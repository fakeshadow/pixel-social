use std::{
    time::Duration,
    collections::HashMap,
};
use futures::future::{ok as ft_ok, err as ft_err, Either, join_all};

use actix::prelude::{
    ActorFuture,
    AsyncContext,
    Context,
    Future,
    Handler,
    Message,
    ResponseFuture,
    WrapFuture,
};
use chrono::{Utc, NaiveDateTime};
use redis::{Client, cmd, pipe};

use crate::{
    CacheService,
    CacheUpdateService,
};
use crate::model::{
    actors::SharedConn,
    errors::ServiceError,
    category::Category,
    post::Post,
    topic::Topic,
    user::User,
    cache::{FromHashSetMulti, Parser, SortHash},
    common::{GetSelfId, GetUserId},
    mail::Mail,
};

// page offsets of list query
const LIMIT: isize = 20;
// use LEX_BASE minus pid and tid before adding to zrange.
const LEX_BASE: u32 = std::u32::MAX;
// list_pop update interval time gap in seconds
const LIST_TIME_GAP: Duration = Duration::from_secs(5);
// trim list_pop interval time gap
const TRIM_LIST_TIME_GAP: Duration = Duration::from_secs(1800);
// hash life is expire time of topic and post hash in seconds.
const HASH_LIFE: usize = 172800;
// mail life is expire time of mail hash in seconds
const MAIL_LIFE: usize = 3600;

impl CacheUpdateService {
    pub fn update_list_pop(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(LIST_TIME_GAP, move |act, ctx| {
            let f =
                get_categories(act.cache.as_ref().unwrap().clone())
                    .into_actor(act)
                    .map_err(|_, _, _| ())
                    .and_then(|cat, act, _| {
                        let conn = act.cache.as_ref().unwrap().clone();
                        let yesterday = Utc::now().timestamp_millis() - 86400000;
                        let mut vec = Vec::new();

                        for c in cat.iter() {
                            // update_list will also update topic count new.
                            vec.push(Either::A(update_list(Some(c.id), yesterday, conn.clone())));
                            vec.push(Either::B(update_post_count(c.id, yesterday, conn.clone())));
                        }
                        vec.push(Either::A(update_list(None, yesterday, conn)));

                        join_all(vec)
                            .into_actor(act)
                            .map_err(|_, _, _| ())
                            .map(|_, _, _| ())
                    });
            ctx.spawn(f);
        });
    }

    pub fn trim_list_pop(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(TRIM_LIST_TIME_GAP, move |act, ctx| {
            let f =
                get_categories(act.cache.as_ref().unwrap().clone())
                    .into_actor(act)
                    .map_err(|_, _, _| ())
                    .and_then(|cat, act, _| {
                        let conn = act.cache.as_ref().unwrap().clone();
                        let yesterday = Utc::now().timestamp_millis() - 86400000;
                        let mut vec = Vec::new();

                        for c in cat.iter() {
                            vec.push(trim_list(Some(c.id), yesterday, conn.clone()));
                        }
                        vec.push(trim_list(None, yesterday, conn));

                        join_all(vec)
                            .into_actor(act)
                            .map_err(|_, _, _| ())
                            .map(|_, _, _| ())
                    });
            ctx.spawn(f);
        });
    }
}

pub struct GetCategoriesCache;

pub enum GetTopicsCache {
    Latest(u32, i64),
    Popular(u32, i64),
    PopularAll(i64),
    Ids(Vec<u32>),
}

pub enum GetTopicCache {
    Old(u32, i64),
    Popular(u32, i64),
}

pub struct GetPostsCache(pub Vec<u32>);

#[derive(Message)]
pub struct AddedTopic(pub Topic);

#[derive(Message)]
pub struct AddedPost(pub Post);

#[derive(Message)]
pub struct AddedCategory(pub Category);

#[derive(Message)]
pub enum UpdateCache<T> {
    Topic(Vec<T>),
    Post(Vec<T>),
    User(Vec<T>),
    Category(Vec<T>),
}

#[derive(Message)]
pub enum DeleteCache {
    Mail(String),
}

#[derive(Message)]
pub struct AddMail(pub Mail);

pub struct RemoveCategoryCache(pub u32);

pub struct ActivateUser(pub String);

impl Message for ActivateUser {
    type Result = Result<u32, ServiceError>;
}

impl Message for GetCategoriesCache {
    type Result = Result<Vec<Category>, ServiceError>;
}

impl Message for GetTopicsCache {
    type Result = Result<(Vec<Topic>, Vec<u32>), ServiceError>;
}

impl Message for GetTopicCache {
    type Result = Result<(Vec<Post>, Vec<u32>), ServiceError>;
}

impl Message for GetPostsCache {
    type Result = Result<(Vec<Post>, Vec<User>), ServiceError>;
}

impl Message for RemoveCategoryCache {
    type Result = Result<(), ServiceError>;
}

impl<T> Handler<UpdateCache<T>> for CacheService
    where T: GetSelfId + SortHash + 'static {
    type Result = ();

    fn handle(&mut self, msg: UpdateCache<T>, ctx: &mut Self::Context) -> Self::Result {
        let conn = self.get_conn();

        match msg {
            UpdateCache::Topic(vec) => ctx.spawn(build_hmsets_expire(conn, vec, "topic")
                .into_actor(self)
                .map_err(|_, _, _| ())
                .map(|_, _, _| ())),
            UpdateCache::Post(vec) => ctx.spawn(build_hmsets_expire(conn, vec, "post")
                .into_actor(self)
                .map_err(|_, _, _| ())
                .map(|_, _, _| ())),
            UpdateCache::User(vec) => ctx.spawn(build_hmsets(conn, vec, "user")
                .into_actor(self)
                .map_err(|_, _, _| ())
                .map(|_, _, _| ())),
            UpdateCache::Category(vec) => ctx.spawn(build_hmsets(conn, vec, "category")
                .into_actor(self)
                .map_err(|_, _, _| ())
                .map(|_, _, _| ()))
        };
    }
}

impl Handler<GetCategoriesCache> for CacheService {
    type Result = ResponseFuture<Vec<Category>, ServiceError>;

    fn handle(&mut self, _: GetCategoriesCache, _: &mut Self::Context) -> Self::Result {
        Box::new(get_categories(self.get_conn()))
    }
}

impl Handler<AddedTopic> for CacheService {
    type Result = ();

    fn handle(&mut self, msg: AddedTopic, ctx: &mut Self::Context) -> Self::Result {
        let t = msg.0;
        let tid = t.id;
        let cid = t.category_id;
        let time = t.created_at.timestamp_millis();

        let mut pipe = pipe();
        pipe.atomic();
        let key = format!("topic:{}:set", t.self_id());
        pipe.cmd("HMSET").arg(key.as_str()).arg(t.sort_hash()).ignore()
            .cmd("EXPIRE").arg(key.as_str()).arg(HASH_LIFE).ignore()
            .cmd("HINCRBY").arg(&format!("topic:{}:set_perm", tid)).arg("reply_count").arg(0).ignore()

            .cmd("HINCRBY").arg(&format!("category:{}:set", cid)).arg("topic_count").arg(1).ignore()
            .cmd("lpush").arg(&format!("category:{}:list", cid)).arg(tid).ignore()

            .cmd("ZADD").arg("category:all:topics_time").arg(time).arg(tid).ignore()
            .cmd("ZADD").arg(&format!("category:{}:topics_time", cid)).arg(time).arg(tid).ignore()
            .cmd("ZINCRBY").arg("category:all:topics_reply").arg(0).arg(tid).ignore()
            .cmd("ZINCRBY").arg(&format!("category:{}:topics_reply", cid)).arg(0).arg(tid).ignore();

        let f = pipe
            .query_async(self.get_conn())
            .into_actor(self)
            .map_err(|_, _, _| ())
            .map(|(_, ()), _, _| ());

        ctx.spawn(f);
    }
}

impl Handler<AddedPost> for CacheService {
    type Result = ();

    fn handle(&mut self, msg: AddedPost, ctx: &mut Self::Context) -> Self::Result {
        let p = msg.0;
        let cid = p.category_id;
        let tid = p.topic_id;
        let pid = p.id;
        let time = p.created_at;
        let time_string = time.to_string();

        let mut pipe = pipe();
        pipe.atomic();

        let post_key = format!("post:{}:set", pid);
        let time = time.timestamp_millis();

        pipe.cmd("HMSET").arg(post_key.as_str()).arg(p.sort_hash()).ignore()
            .cmd("EXPIRE").arg(post_key.as_str()).arg(HASH_LIFE).ignore()
            .cmd("HINCRBY").arg(&format!("post:{}:set_perm", pid)).arg("reply_count").arg(0).ignore()

            .cmd("HINCRBY").arg(&format!("category:{}:set", cid)).arg("post_count").arg(1).ignore()
            .cmd("lrem").arg(&format!("category:{}:list", cid)).arg(1).arg(tid).ignore()
            .cmd("lpush").arg(&format!("category:{}:list", cid)).arg(tid).ignore()
            .cmd("rpush").arg(&format!("topic:{}:list", tid)).arg(pid).ignore()

            .cmd("HINCRBY").arg(&format!("topic:{}:set_perm", tid)).arg("reply_count").arg(1).ignore()
            .cmd("HSET").arg(&format!("topic:{}:set", tid)).arg("last_reply_time").arg(&time_string).ignore()

            .cmd("ZINCRBY").arg(&format!("topic:{}:posts_reply", tid)).arg(0).arg(LEX_BASE - pid).ignore()

            .cmd("ZADD").arg(&format!("category:{}:topics_time", cid)).arg("XX").arg(time).arg(tid).ignore()
            .cmd("ZADD").arg("category:all:topics_time").arg("XX").arg(time).arg(tid).ignore()
            .cmd("ZINCRBY").arg(&format!("category:{}:topics_reply", cid)).arg(1).arg(tid).ignore()
            .cmd("ZINCRBY").arg("category:all:topics_reply").arg(1).arg(tid).ignore()

            .cmd("ZADD").arg(&format!("category:{}:posts_time", cid)).arg(time).arg(pid).ignore();

        if let Some(pid) = p.post_id {
            pipe.cmd("HSET").arg(&format!("post:{}:set", pid)).arg("last_reply_time").arg(&time_string).ignore()
                .cmd("HINCRBY").arg(&format!("post:{}:set_perm", pid)).arg("reply_count").arg(1).ignore()
                .cmd("ZINCRBY").arg(&format!("topic:{}:posts_reply", tid)).arg(1).arg(LEX_BASE - pid).ignore();
        }

        ctx.spawn(pipe
            .query_async(self.get_conn())
            .into_actor(self)
            .map_err(|_, _, _| ())
            .map(|(_, ()), _, _| ()));
    }
}

impl Handler<AddedCategory> for CacheService {
    type Result = ();

    fn handle(&mut self, msg: AddedCategory, ctx: &mut Self::Context) -> Self::Result {
        let c = msg.0;
        let mut pipe = pipe();
        pipe.atomic();
        pipe.cmd("rpush").arg("category_id:meta").arg(c.id);
        pipe.cmd("HMSET").arg(&format!("category:{}:set", c.id)).arg(c.sort_hash());
        let f = pipe
            .query_async(self.get_conn())
            .into_actor(self)
            .map_err(|_, _, _| ())
            .map(|(_, ()), _, _| ());

        ctx.spawn(f);
    }
}

impl Handler<GetTopicCache> for CacheService {
    type Result = ResponseFuture<(Vec<Post>, Vec<u32>), ServiceError>;

    fn handle(&mut self, msg: GetTopicCache, _: &mut Self::Context) -> Self::Result {
        match msg {
            GetTopicCache::Old(tid, page) => Box::new(
                topics_posts_from_list(
                    page,
                    &format!("topic:{}:list", tid),
                    "post",
                    self.get_conn())),
            GetTopicCache::Popular(tid, page) => Box::new(
                cmd("zrevrangebyscore")
                    .arg(&format!("topic:{}:posts_reply", tid))
                    .arg("+inf")
                    .arg("-inf")
                    .arg("LIMIT")
                    .arg(((page - 1) * 20) as usize)
                    .arg(20)
                    .query_async(self.get_conn())
                    .from_err()
                    .and_then(move |(conn, ids): (SharedConn, Vec<u32>)| {
                        if ids.len() == 0 {
                            return Either::A(ft_err(ServiceError::NoContent));
                        }
                        let ids = ids.into_iter().map(|i|LEX_BASE - i).collect();
                        Either::B(get_cache_multi_with_id(&ids, "post", conn)
                            .map_err(|_| ServiceError::IdsFromCache(ids))
                            .map(|(v, i, _)| (v, i)))
                    }))
        }
    }
}

impl Handler<GetPostsCache> for CacheService {
    type Result = ResponseFuture<(Vec<Post>, Vec<User>), ServiceError>;

    fn handle(&mut self, msg: GetPostsCache, _: &mut Self::Context) -> Self::Result {
        let f =
            get_cache_multi_with_id(&msg.0, "post", self.get_conn())
                .and_then(|(p, mut uids, conn)| {
                    uids.sort();
                    uids.dedup();
                    get_users(conn, &uids).map(|u| (p, u))
                });
        Box::new(f)
    }
}

//ToDo: move this handler to delete cache enum
impl Handler<RemoveCategoryCache> for CacheService {
    type Result = ResponseFuture<(), ServiceError>;

    fn handle(&mut self, msg: RemoveCategoryCache, _: &mut Self::Context) -> Self::Result {
        let key = format!("category:{}:set", msg.0);
        let fields = ["id", "name", "topic_count", "post_count", "subscriber_count", "thumbnail"];

        let mut pipe = pipe();
        pipe.atomic();
        pipe.cmd("lrem").arg(1).arg("category_id:meta");

        for f in fields.to_vec() {
            pipe.cmd("hdel").arg(&key).arg(f);
        }

        Box::new(pipe.query_async(self.get_conn())
            .from_err()
            .map(|(_, _): (_, usize)| ()))
    }
}

impl Handler<DeleteCache> for CacheService {
    type Result = ();

    fn handle(&mut self, msg: DeleteCache, ctx: &mut Self::Context) -> Self::Result {
        let (key, fields) = match msg {
            DeleteCache::Mail(uuid) => {
                let fields = ["user_id", "uuid"];
                (uuid, fields)
            }
        };

        let mut pipe = pipe();
        pipe.atomic();

        for f in fields.to_vec() {
            pipe.cmd("hdel").arg(&key).arg(f);
        }

        ctx.spawn(pipe
            .query_async(self.get_conn())
            .into_actor(self)
            .map_err(|_, _, _| ())
            .map(|(_, _): (_, usize), _, _| ()));
    }
}

impl Handler<AddMail> for CacheService {
    type Result = ();

    fn handle(&mut self, msg: AddMail, ctx: &mut Self::Context) -> Self::Result {
        let mail = msg.0;
        //ToDo: add stringify error handler.

        if let Some(s) = serde_json::to_string(&mail).ok() {
            let mut pipe = pipe();
            pipe.atomic();
            pipe.cmd("ZADD").arg("mail_queue").arg(mail.user_id).arg(s.as_str());
            pipe.cmd("HMSET").arg(&mail.uuid).arg(mail.sort_hash());
            pipe.cmd("EXPIRE").arg(&mail.uuid).arg(MAIL_LIFE);

            let f = pipe
                .query_async(self.get_conn())
                .into_actor(self)
                .map_err(|_, _, _| ())
                .map(|(_, ()), _, _| ());

            ctx.spawn(f);
        }
    }
}

impl Handler<ActivateUser> for CacheService {
    type Result = ResponseFuture<u32, ServiceError>;

    fn handle(&mut self, msg: ActivateUser, _: &mut Self::Context) -> Self::Result {
        let f = cmd("HGETALL")
            .arg(&msg.0)
            .query_async(self.get_conn())
            .from_err()
            .and_then(move |(_, hm): (_, HashMap<String, String>)| {
                let m = hm.parse::<Mail>()?;
                if msg.0 == m.uuid {
                    Ok(m.user_id)
                } else {
                    Err(ServiceError::Unauthorized)
                }
            });
        Box::new(f)
    }
}

impl Handler<GetTopicsCache> for CacheService {
    type Result = ResponseFuture<(Vec<Topic>, Vec<u32>), ServiceError>;

    fn handle(&mut self, msg: GetTopicsCache, _: &mut Self::Context) -> Self::Result {
        match msg {
            GetTopicsCache::Popular(id, page) =>
                Box::new(topics_posts_from_list(
                    page,
                    &format!("category:{}:list_pop", id),
                    "topic",
                    self.get_conn())),
            GetTopicsCache::PopularAll(page) =>
                Box::new(topics_posts_from_list(
                    page,
                    "category:all:list_pop",
                    "topic",
                    self.get_conn())),
            GetTopicsCache::Latest(id, page) =>
                Box::new(topics_posts_from_list(
                    page,
                    &format!("category:{}:list", id),
                    "topic",
                    self.get_conn())),
            GetTopicsCache::Ids(ids) =>
                Box::new(get_cache_multi_with_id(&ids, "topic", self.get_conn())
                    .map_err(|_| ServiceError::IdsFromCache(ids))
                    .map(|(v, i, _)| (v, i)))
        }
    }
}

fn topics_posts_from_list<T>(
    page: i64,
    list_key: &str,
    set_key: &'static str,
    conn: SharedConn,
) -> impl Future<Item=(Vec<T>, Vec<u32>), Error=ServiceError>
    where T: GetUserId + FromHashSetMulti {
    let start = (page as isize - 1) * 20;

    cmd("lrange")
        .arg(list_key)
        .arg(start)
        .arg(start + LIMIT - 1)
        .query_async(conn)
        .from_err()
        .and_then(move |(conn, ids): (SharedConn, Vec<u32>)| {
            if ids.len() == 0 {
                return Either::A(ft_err(ServiceError::NoContent));
            }
            Either::B(get_cache_multi_with_id(&ids, set_key, conn)
                .map_err(|_| ServiceError::IdsFromCache(ids))
                .map(|(v, i, _)| (v, i))
            )
        })
}


fn update_post_count(
    cid: u32,
    yesterday: i64,
    conn: SharedConn,
) -> impl Future<Item=(), Error=ServiceError> {
    let time_key = format!("category:{}:posts_time", cid);
    let set_key = format!("category:{}:set", cid);

    cmd("ZCOUNT")
        .arg(&time_key)
        .arg(yesterday)
        .arg("+inf")
        .query_async(conn)
        .from_err()
        .and_then(move |(conn, count): (_, u32)| {
            cmd("HMSET")
                .arg(&set_key)
                .arg(&[("post_count_new", count)])
                .query_async(conn)
                .from_err()
                .map(|(_, ())| ())
        })
}

fn update_list(
    cid: Option<u32>,
    yesterday: i64,
    conn: SharedConn,
) -> impl Future<Item=(), Error=ServiceError> {
    let (list_key, time_key, reply_key, set_key) = match cid.as_ref() {
        Some(cid) => (
            format!("category:{}:list_pop", cid),
            format!("category:{}:topics_time", cid),
            format!("category:{}:topics_reply", cid),
            Some(format!("category:{}:set", cid))),
        None => (
            "category:all:list_pop".to_owned(),
            "category:all:topics_time".to_owned(),
            "category:all:topics_reply".to_owned(),
            None
        )
    };

    let mut pip = pipe();
    pip.atomic();
    pip.cmd("ZREVRANGEBYSCORE")
        .arg(&time_key)
        .arg("+inf")
        .arg(yesterday)
        .arg("WITHSCORES")
        .cmd("ZREVRANGEBYSCORE")
        .arg(&reply_key)
        .arg("+inf")
        .arg("-inf")
        .arg("WITHSCORES");

    pip.query_async(conn)
        .from_err()
        .and_then(move |(conn, (tids, mut counts)): (_, (HashMap<u32, i64>, Vec<(u32, u32)>))| {
            use std::cmp::Ordering;
            counts.sort_by(|(a0, a1), (b0, b1)| {
                if a1 == b1 {
                    if let Some(a) = tids.get(a0) {
                        if let Some(b) = tids.get(b0) {
                            if a > b {
                                return Ordering::Less;
                            } else if a < b {
                                return Ordering::Greater;
                            };
                        }
                    }
                    Ordering::Equal
                } else {
                    Ordering::Greater
                }
            });

            let vec = counts.into_iter().map(|(id, _)| id).collect::<Vec<u32>>();

            let mut should_update = false;
            let mut pip = pipe();
            pip.atomic();

            if let Some(key) = set_key {
                pip.cmd("HSET")
                    .arg(&key)
                    .arg("topic_count_new")
                    .arg(tids.len())
                    .ignore();
                should_update = true;
            }

            if vec.len() > 0 {
                pip.cmd("del")
                    .arg(&list_key)
                    .ignore()
                    .cmd("rpush")
                    .arg(&list_key)
                    .arg(vec)
                    .ignore();
                should_update = true;
            }

            if should_update {
                Either::A(pip
                    .query_async(conn)
                    .from_err()
                    .map(|(_, ())| ()))
            } else {
                Either::B(ft_ok(()))
            }
        })
}

fn trim_list(
    cid: Option<u32>,
    yesterday: i64,
    conn: SharedConn,
) -> impl Future<Item=(), Error=ServiceError> {
    let (time_key, reply_key, time_key_post) = match cid.as_ref() {
        Some(cid) => (
            format!("category:{}:topics_time", cid),
            format!("category:{}:topics_reply", cid),
            Some(format!("category:{}:posts_time", cid))),
        None => (
            "category:all:topics_time".to_owned(),
            "category:all:topics_reply".to_owned(),
            None
        )
    };

    let mut pip = pipe();
    pip.atomic();
    pip.cmd("zrevrangebyscore")
        .arg(&time_key)
        .arg("+inf")
        .arg(yesterday)
        .cmd("ZREVRANGEBYSCORE")
        .arg(&reply_key)
        .arg("+inf")
        .arg("-inf")
        .arg("WITHSCORES");

    pip.query_async(conn)
        .from_err()
        .and_then(move |(conn, (tids, counts)): (_, (Vec<u32>, Vec<(u32, u32)>))| {
            let mut pipe = pipe();
            for (tid, _) in counts.into_iter() {
                if !tids.contains(&tid) {
                    pipe.cmd("zrem").arg(&reply_key).arg(tid).ignore();
                }
            }
            pipe.cmd("ZREMRANGEBYSCORE").arg(&time_key).arg(yesterday).arg("-inf").ignore();

            if let Some(key) = time_key_post {
                pipe.cmd("ZREMRANGEBYSCORE").arg(&key).arg(yesterday).arg("-inf").ignore();
            }

            pipe.query_async(conn)
                .from_err()
                .map(|(_, ())| ())
        })
}

pub struct GetUsersCache(pub Vec<u32>);

impl Message for GetUsersCache {
    type Result = Result<Vec<User>, Vec<u32>>;
}

impl Handler<GetUsersCache> for CacheService {
    type Result = ResponseFuture<Vec<User>, Vec<u32>>;

    fn handle(&mut self, mut msg: GetUsersCache, _: &mut Self::Context) -> Self::Result {
        msg.0.sort();
        msg.0.dedup();
        Box::new(get_users(self.get_conn(), &msg.0)
            .map_err(|_| msg.0))
    }
}

fn get_categories(
    conn: SharedConn,
) -> impl Future<Item=Vec<Category>, Error=ServiceError> {
    cmd("lrange")
        .arg("category_id:meta")
        .arg(0)
        .arg(-1)
        .query_async(conn)
        .from_err()
        .and_then(|(conn, vec): (_, Vec<u32>)| get_hmsets(conn, &vec, "category"))
        .and_then(|(_, vec, _): (_, Vec<HashMap<String, String>>, _)| vec
            .iter()
            .map(|hash| hash
                .parse::<Category>())
            .collect::<Result<Vec<Category>, ServiceError>>())
}

// consume connection as users usually the last query.
pub fn get_users(
    conn: SharedConn,
    uids: &Vec<u32>,
) -> impl Future<Item=Vec<User>, Error=ServiceError> {
    get_hmsets(conn, &uids, "user")
        .and_then(move |(target_len, hm, _)| {
            let mut u = Vec::new();
            for v in hm.iter() {
                if let Some(user) = v.parse::<User>().ok() {
                    u.push(user);
                }
            };
            if u.len() != target_len {
                return Err(ServiceError::InternalServerError);
            };
            Ok(u)
        })
}

// helper functions
pub fn build_hmsets<T>(
    conn: SharedConn,
    vec: Vec<T>,
    key: &'static str,
) -> impl Future<Item=(), Error=ServiceError>
    where T: GetSelfId + SortHash {
    let mut pipe = pipe();
    pipe.atomic();
    for v in vec.iter() {
        pipe.cmd("HMSET")
            .arg(&format!("{}:{}:set", key, v.self_id()))
            .arg(v.sort_hash());
    }
    pipe.query_async(conn)
        .from_err()
        .map(|(_, ())| ())
}

pub fn build_hmsets_expire<T>(
    conn: SharedConn,
    vec: Vec<T>,
    key: &'static str,
) -> impl Future<Item=(), Error=ServiceError>
    where T: GetSelfId + SortHash {
    let mut pipe = pipe();
    pipe.atomic();
    for v in vec.iter() {
        let key_expire = format!("{}:{}:set", key, v.self_id());
        pipe.cmd("HMSET")
            .arg(key_expire.as_str())
            .arg(v.sort_hash())
            .ignore()
            .cmd("expire")
            .arg(key_expire.as_str())
            .arg(HASH_LIFE)
            .ignore();
    }
    pipe.query_async(conn)
        .from_err()
        .map(|(_, ())| ())
}

pub fn build_list(
    conn: SharedConn,
    vec: Vec<u32>,
    // pass lpush or rpush as cmd
    cmd: &'static str,
    key: String,
) -> impl Future<Item=(), Error=ServiceError> {
    let mut pipe = pipe();
    pipe.atomic();
    for v in vec.into_iter() {
        pipe.cmd(cmd).arg(&key).arg(v);
    }
    pipe.query_async(conn)
        .from_err()
        .map(|(_, ())| ())
}

fn get_hmsets<'a, T>(
    conn: SharedConn,
    vec: &'a Vec<T>,
    key: &'a str,
) -> impl Future<Item=(usize, Vec<HashMap<String, String>>, SharedConn), Error=ServiceError>
    where T: std::fmt::Display {
    let mut pipe = pipe();
    pipe.atomic();
    for v in vec.iter() {
        pipe.cmd("HGETALL").arg(&format!("{}:{}:set", key, v));
    }
    let len = vec.len();
    pipe.query_async(conn)
        .from_err()
        .map(move |(conn, hm)| (len, hm, conn))
}

fn get_cache_multi_with_id<'a, T>(
    ids: &'a Vec<u32>,
    set_key: &'a str,
    conn: SharedConn,
) -> impl Future<Item=(Vec<T>, Vec<u32>, SharedConn), Error=ServiceError>
    where T: GetUserId + FromHashSetMulti {
    get_hmsets_multi(conn, ids, set_key)
        .and_then(|(len, hm, conn): (usize, Vec<(HashMap<String, String>, HashMap<String, String>)>, SharedConn)| {
            use crate::model::cache::ParserMulti;
            let mut res: Vec<T> = Vec::with_capacity(20);
            let mut uids: Vec<u32> = Vec::with_capacity(21);
            for h in hm.iter() {
                if let Some(t) = h.parse::<T>().ok() {
                    let uid = t.get_user_id();
                    if !uids.contains(&uid) {
                        uids.push(uid);
                    }
                    res.push(t);
                }
            }
            if res.len() != len {
                return Err(ServiceError::InternalServerError);
            };
            Ok((res, uids, conn))
        })
}

fn get_hmsets_multi<'a, T>(
    conn: SharedConn,
    vec: &'a Vec<T>,
    key: &'a str,
) -> impl Future<Item=(usize, Vec<(HashMap<String, String>, HashMap<String, String>)>, SharedConn), Error=ServiceError>
    where T: std::fmt::Display {
    let mut pipe = pipe();
    pipe.atomic();
    for v in vec.iter() {
        pipe.cmd("HGETALL").arg(&format!("{}:{}:set", key, v))
            .cmd("HGETALL").arg(&format!("{}:{}:set_perm", key, v));
    }
    let len = vec.len();
    pipe.query_async(conn)
        .from_err()
        .map(move |(conn, hm)| (len, hm, conn))
}

pub fn from_mail_queue(
    conn: SharedConn,
) -> impl Future<Item=(SharedConn, String), Error=ServiceError> {
    cmd("zrange")
        .arg("mail_queue")
        .arg(0)
        .arg(0)
        .query_async(conn)
        .from_err()
        .and_then(move |(conn, mut s): (_, Vec<String>)| {
            let s = s.pop().ok_or(ServiceError::InternalServerError)?;
            Ok((conn, s))
        })
}

pub fn delete_mail_queue(
    key: &str,
    conn: SharedConn,
) -> impl Future<Item=(), Error=ServiceError> {
    cmd("zrem")
        .arg("mail_queue")
        .arg(key)
        .query_async(conn)
        .from_err()
        .map(|(_, ())| ())
}

// startup helper fn
pub fn clear_cache(redis_url: &str) -> Result<(), ServiceError> {
    let client = Client::open(redis_url).expect("failed to connect to redis server");
    let mut conn = client.get_connection().expect("failed to get redis connection");
    Ok(redis::cmd("flushall").query(&mut conn)?)
}

pub fn build_topics_cache_list(
    vec: Vec<(u32, u32, u32, NaiveDateTime)>,
    conn: SharedConn,
) -> impl Future<Item=(), Error=ServiceError> {
    let mut pipe = pipe();
    pipe.atomic();

    for (tid, cid, count, time) in vec.into_iter() {
        let time = time.timestamp_millis();
        pipe.cmd("ZADD").arg("category:all:topics_time").arg(time).arg(tid).ignore()
            .cmd("ZADD").arg("category:all:topics_reply").arg(count).arg(tid).ignore()
            .cmd("ZADD").arg(&format!("category:{}:topics_time", cid)).arg(time).arg(tid).ignore()
            .cmd("ZADD").arg(&format!("category:{}:topics_reply", cid)).arg(count).arg(tid).ignore()
            // set topic's reply count to perm key that never expire.
            .cmd("HSET").arg(&format!("topic:{}:set_perm", tid)).arg("reply_count").arg(count).ignore();
    }

    pipe.query_async(conn)
        .from_err()
        .map(|(_, ())| ())
}

pub fn build_posts_cache_list(
    vec: Vec<(u32, u32, u32)>,
    conn: SharedConn,
) -> impl Future<Item=(), Error=ServiceError> {
    let mut pipe = pipe();
    pipe.atomic();

    for (tid, pid, count) in vec.into_iter() {
        pipe.cmd("ZADD").arg(&format!("topic:{}:posts_reply", tid)).arg(count).arg(LEX_BASE - pid).ignore()
            // set post's reply count to perm key that never expire.
            .cmd("HSET").arg(&format!("post:{}:set_perm", pid)).arg("reply_count").arg(count).ignore();
    }

    pipe.query_async(conn)
        .from_err()
        .map(|(_, ())| ())
}
