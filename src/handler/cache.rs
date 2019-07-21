use std::{
    time::Duration,
    collections::HashMap,
};
use futures::future::{err as fut_err, Either, join_all};

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
use chrono::{NaiveDateTime, Utc};
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

lazy_static! {
    static ref BASETIME:NaiveDateTime = NaiveDateTime::parse_from_str("2019-07-20 2:33:33.666666", "%Y-%m-%d %H:%M:%S%.f").unwrap();
}

// page offsets of list query
const LIMIT: isize = 20;

// add to pid when zadd topic:{}:post_rely
const POST_LEX_BASE: u32 = 1000000000;
// list_pop update interval time gap in seconds
const LIST_TIME_GAP: Duration = Duration::from_secs(10);
// trim list_pop interval time gap
const TRIM_LIST_TIME_GAP: Duration = Duration::from_secs(3600);
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
                        let yesterday = Utc::now().timestamp_millis() - BASETIME.timestamp_millis() - 86400000;
                        let mut vec = Vec::new();

                        for c in cat.iter() {
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
                        let yesterday = Utc::now().timestamp_millis() - BASETIME.timestamp_millis() - 86400000;
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
}

pub enum GetTopicCache {
    Old(u32, i64),
    Popular(u32, i64),
}

pub struct GetPostsCache(pub Vec<u32>);

pub struct AddedTopic(pub Topic);

pub struct AddedPost(pub Post);

pub struct AddedCategory(pub Category);

pub struct RemoveCategoryCache(pub u32);

pub enum UpdateCache<T> {
    Topic(Vec<T>),
    Post(Vec<T>),
    User(Vec<T>),
    Category(Vec<T>),
}

pub enum DeleteCache {
    Mail(String),
}

pub struct AddMail(pub Mail);

pub struct ActivateUser(pub String);

impl Message for ActivateUser {
    type Result = Result<u32, ServiceError>;
}

impl Message for GetCategoriesCache {
    type Result = Result<Vec<Category>, ServiceError>;
}

impl Message for GetTopicsCache {
    type Result = Result<(Vec<Topic>, Vec<User>), Option<Vec<u32>>>;
}

impl Message for GetTopicCache {
    type Result = Result<(Topic, Vec<Post>, Vec<User>), Option<Vec<u32>>>;
}

impl Message for GetPostsCache {
    type Result = Result<(Vec<Post>, Vec<User>), ServiceError>;
}

impl Message for AddedTopic {
    type Result = Result<(), ServiceError>;
}

impl Message for AddedPost {
    type Result = Result<(), ServiceError>;
}

impl Message for AddedCategory {
    type Result = Result<(), ServiceError>;
}

impl Message for AddMail {
    type Result = Result<(), ServiceError>;
}

impl Message for RemoveCategoryCache {
    type Result = Result<(), ServiceError>;
}

impl Message for DeleteCache {
    type Result = Result<(), ServiceError>;
}

impl<T> Message for UpdateCache<T> {
    type Result = Result<(), ServiceError>;
}

impl<T> Handler<UpdateCache<T>> for CacheService
    where T: GetSelfId + SortHash + 'static {
    type Result = ResponseFuture<(), ServiceError>;

    fn handle(&mut self, msg: UpdateCache<T>, _: &mut Self::Context) -> Self::Result {
        let conn = self.get_conn();

        match msg {
            UpdateCache::Topic(vec) => Box::new(build_hmsets_expire(conn, vec, "topic")),
            UpdateCache::Post(vec) => Box::new(build_hmsets_expire(conn, vec, "post")),
            UpdateCache::User(vec) => Box::new(build_hmsets(conn, vec, "user")),
            UpdateCache::Category(vec) => Box::new(build_hmsets(conn, vec, "category")),
        }
    }
}

impl Handler<GetCategoriesCache> for CacheService {
    type Result = ResponseFuture<Vec<Category>, ServiceError>;

    fn handle(&mut self, _: GetCategoriesCache, _: &mut Self::Context) -> Self::Result {
        Box::new(get_categories(self.get_conn()))
    }
}

impl Handler<AddedTopic> for CacheService {
    type Result = ResponseFuture<(), ServiceError>;

    fn handle(&mut self, msg: AddedTopic, _: &mut Self::Context) -> Self::Result {
        let t = msg.0;
        let tid = t.id;
        let cid = t.category_id;
        let time = t.last_reply_time.timestamp_millis() - BASETIME.timestamp_millis();
        let count = t.reply_count;

        let mut pipe = pipe();
        pipe.atomic();
        let key = format!("topic:{}:set", t.self_id());
        pipe.cmd("HMSET").arg(key.as_str()).arg(t.sort_hash()).ignore()
            .cmd("EXPIRE").arg(key.as_str()).arg(HASH_LIFE).ignore()
            .cmd("HINCRBY").arg(&format!("category:{}:set", cid)).arg("topic_count").arg(1).ignore()
            .cmd("lpush").arg(&format!("category:{}:list", cid)).arg(tid).ignore()
            .cmd("ZADD").arg("category:all:topics_time").arg(time).arg(tid).ignore()
            .cmd("ZADD").arg("category:all:topics_reply").arg(count).arg(tid).ignore()
            .cmd("ZADD").arg(&format!("category:{}:topics_time", cid)).arg(time).arg(tid).ignore()
            .cmd("ZADD").arg(&format!("category:{}:topics_reply", cid)).arg(count).arg(tid).ignore();

        Box::new(pipe
            .query_async(self.get_conn())
            .from_err()
            .map(|(_, ())| ()))
    }
}

impl Handler<AddedPost> for CacheService {
    type Result = ResponseFuture<(), ServiceError>;

    fn handle(&mut self, msg: AddedPost, _: &mut Self::Context) -> Self::Result {
        let p = msg.0;
        let cid = p.category_id;
        let tid = p.topic_id;
        let pid = p.id;
        let count = p.reply_count;
        let time = p.last_reply_time;
        let time_string = time.to_string();

        let mut pipe = pipe();
        pipe.atomic();

        let post_key = format!("post:{}:set", pid);
        let time = time.timestamp_millis() - BASETIME.timestamp_millis();

        pipe.cmd("HINCRBY").arg(&format!("category:{}:set", cid)).arg("post_count").arg(1).ignore()
            .cmd("lrem").arg(&format!("category:{}:list", cid)).arg(1).arg(tid).ignore()
            .cmd("lpush").arg(&format!("category:{}:list", cid)).arg(tid).ignore()
            .cmd("rpush").arg(&format!("topic:{}:list", tid)).arg(pid).ignore()
            .cmd("HMSET").arg(post_key.as_str()).arg(p.sort_hash()).ignore()
            .cmd("EXPIRE").arg(post_key.as_str()).arg(HASH_LIFE).ignore()
            .cmd("HINCRBY").arg(&format!("topic:{}:set_perm", tid)).arg("reply_count").arg(1).ignore()
            .cmd("HSET").arg(&format!("topic:{}:set", tid)).arg("last_reply_time").arg(&time_string).ignore()
            .cmd("ZADD").arg(&format!("topic:{}:posts_reply", tid)).arg(count).arg(POST_LEX_BASE - pid).ignore()
            .cmd("ZADD").arg(&format!("category:{}:topics_time", cid)).arg("XX").arg(time).arg(tid).ignore()
            .cmd("ZINCRBY").arg(&format!("category:{}:topics_reply", cid)).arg(1).arg(tid).ignore()
            .cmd("ZADD").arg("category:all:topics_time").arg("XX").arg(time).arg(tid).ignore()
            .cmd("ZINCRBY").arg("category:all:topics_reply").arg(1).arg(tid).ignore()
            .cmd("ZADD").arg(&format!("category:{}:posts_time", cid)).arg(time).arg(pid).ignore();

        if let Some(pid) = p.post_id {
            pipe.cmd("HSET").arg(&format!("post:{}:set", pid)).arg("last_reply_time").arg(&time_string).ignore()
                .cmd("HINCRBY").arg(&format!("post:{}:set_perm", pid)).arg("reply_count").arg(1).ignore()
                .cmd("ZINCRBY").arg(&format!("topic:{}:posts_reply", tid)).arg(1).arg(POST_LEX_BASE - pid).ignore();
        }

        Box::new(pipe
            .query_async(self.get_conn())
            .from_err()
            .map(|(_, ())| ()))
    }
}

impl Handler<AddedCategory> for CacheService {
    type Result = ResponseFuture<(), ServiceError>;

    fn handle(&mut self, msg: AddedCategory, _: &mut Self::Context) -> Self::Result {
        let c = msg.0;
        let mut pipe = pipe();
        pipe.atomic();
        pipe.cmd("rpush").arg("category_id:meta").arg(c.id);
        pipe.cmd("HMSET").arg(&format!("category:{}:set", c.id)).arg(c.sort_hash());
        let f = pipe
            .query_async(self.get_conn())
            .from_err()
            .map(|(_, ())| ());

        Box::new(f)
    }
}

impl Handler<GetTopicCache> for CacheService {
    type Result = ResponseFuture<(Topic, Vec<Post>, Vec<User>), Option<Vec<u32>>>;

    fn handle(&mut self, msg: GetTopicCache, _: &mut Self::Context) -> Self::Result {
        match msg {
            GetTopicCache::Old(tid, page) => {
                let list_key = format!("topic:{}:list", tid);
                let start = (page as isize - 1) * 20;
                let f = cmd("lrange")
                    .arg(list_key)
                    .arg(start)
                    .arg(start + LIMIT - 1)
                    .query_async(self.get_conn())
                    .map_err(|_| None)
                    .and_then(move |(conn, ids): (SharedConn, Vec<u32>)| {
                        if ids.len() == 0 {
                            return Either::A(fut_err(None));
                        }
                        let e = Some(ids.clone());

                        Either::B(get_cache_multi_with_id(&ids, "post", conn)
                            .map_err(|_| e)
                            .and_then(move |(p, mut uids, conn)| {
                                let e = Some(ids.clone());
                                get_cache_multi_with_id(&vec![tid], "topic", conn)
                                    .map_err(|_| e)
                                    .and_then(move |(mut t, _, conn): (Vec<Topic>, _, _)| {
                                        let t = match t.pop() {
                                            Some(t) => t,
                                            None => return Either::A(fut_err(None))
                                        };
                                        uids.push(t.user_id);
                                        uids.sort();
                                        uids.dedup();
                                        let e = Some(ids.clone());
                                        Either::B(get_users(conn, &uids)
                                            .map_err(|_| e)
                                            .map(|u| (t, p, u)))
                                    })
                            }))
                    });
                Box::new(f)
            }

            GetTopicCache::Popular(tid, page) => {
                let count_key = format!("topic:{}:posts_reply", tid);
                let start = ((page - 1) * 20) as usize;
                let f = cmd("zrevrangebyscore")
                    .arg(&count_key)
                    .arg("+inf")
                    .arg("-inf")
                    .arg("LIMIT")
                    .arg(start)
                    .arg(20)
                    .query_async(self.get_conn())
                    .map_err(|_| None)
                    .and_then(move |(conn, ids): (SharedConn, Vec<u32>)| {
                        if ids.len() == 0 {
                            return Either::A(fut_err(None));
                        }
                        let ids = ids.into_iter().map(|id| POST_LEX_BASE - id).collect::<Vec<u32>>();
                        let e = Some(ids.clone());

                        Either::B(get_cache_multi_with_id(&ids, "post", conn)
                            .map_err(|_| e)
                            .and_then(move |(p, mut uids, conn)| {
                                let e = Some(ids.clone());
                                get_cache_multi_with_id(&vec![tid], "topic", conn)
                                    .map_err(|_| e)
                                    .and_then(move |(mut t, _, conn): (Vec<Topic>, _, _)| {
                                        let t = match t.pop() {
                                            Some(t) => t,
                                            None => return Either::A(fut_err(None))
                                        };
                                        uids.push(t.user_id);
                                        uids.sort();
                                        uids.dedup();
                                        let e = Some(ids.clone());
                                        Either::B(get_users(conn, &uids)
                                            .map_err(|_| e)
                                            .map(|u| (t, p, u)))
                                    })
                            }))
                    });

                Box::new(f)
            }
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
    type Result = ResponseFuture<(), ServiceError>;

    fn handle(&mut self, msg: DeleteCache, _: &mut Self::Context) -> Self::Result {
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

        Box::new(pipe
            .query_async(self.get_conn())
            .from_err()
            .map(|(_, _): (_, usize)| ()))
    }
}

impl Handler<AddMail> for CacheService {
    type Result = ResponseFuture<(), ServiceError>;

    fn handle(&mut self, msg: AddMail, _: &mut Self::Context) -> Self::Result {
        let mail = msg.0;
        //ToDo: add stringify error handler.
        let string = match serde_json::to_string(&mail) {
            Ok(s) => s,
            Err(_) => return Box::new(fut_err(ServiceError::InternalServerError))
        };

        let mut pipe = pipe();
        pipe.atomic();
        pipe.cmd("ZADD").arg("mail_queue").arg(mail.user_id).arg(&string);
        pipe.cmd("HMSET").arg(&mail.uuid).arg(mail.sort_hash());
        pipe.cmd("EXPIRE").arg(&mail.uuid).arg(MAIL_LIFE);

        let f = pipe
            .query_async(self.get_conn())
            .from_err()
            .map(|(_, ())| ());

        Box::new(f)
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
    type Result = ResponseFuture<(Vec<Topic>, Vec<User>), Option<Vec<u32>>>;

    fn handle(&mut self, msg: GetTopicsCache, _: &mut Self::Context) -> Self::Result {
        let (key, page) = match msg {
            GetTopicsCache::Popular(id, page) =>
                (format!("category:{}:list_pop", id), page),
            GetTopicsCache::PopularAll(page) =>
                ("category:all:list_pop".to_owned(), page),
            GetTopicsCache::Latest(id, page) =>
                (format!("category:{}:list", id), page),
        };

        let f =
            topics_posts_from_list(
                page,
                key.as_str(),
                "topic",
                self.get_conn())
                .and_then(|(t, mut uids, conn)| {
                    uids.sort();
                    uids.dedup();
                    get_users(conn, &uids)
                        .map_err(|_| None)
                        .map(|u| (t, u))
                });

        Box::new(f)
    }
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
        .and_then(move |(conn, (tids, mut counts)): (_, (Vec<u32>, Vec<(u32, u32)>))| {
            let len = counts.len();
            let topic_count_new = tids.len();

            let mut temp = Vec::with_capacity(len);
            let mut vec = Vec::with_capacity(len);

            // sort two ranged scores with the last_reply_time desc and reply_count desc.

            for tid in tids.into_iter() {
                for j in 0..counts.len() {
                    if tid == counts[j].0 {
                        let l = temp.len();
                        if l == 0 {
                            vec.push(tid);
                            temp.push(counts.swap_remove(j));
                        } else {
                            for k in 0..l {
                                if counts[j].1 > temp[k].1 {
                                    let k = if k > 1 { k - 1 } else { 0 };
                                    vec.insert(k, tid);
                                    temp.insert(k, counts.swap_remove(j));
                                    break;
                                } else {
                                    vec.push(tid);
                                    temp.push(counts.swap_remove(j));
                                    break;
                                }
                            }
                        };
                        break;
                    }
                }
            }

            let mut pipe = pipe();
            pipe.atomic();

            if vec.len() > 0 {
                pipe.cmd("del")
                    .arg(&list_key)
                    .ignore()
                    .cmd("rpush")
                    .arg(&list_key)
                    .arg(vec)
                    .ignore();
            }

            // update topic_count_new
            if let Some(key) = set_key {
                pipe.cmd("HSET")
                    .arg(&key)
                    .arg("topic_count_new")
                    .arg(topic_count_new)
                    .ignore();
            }

            pipe.query_async(conn)
                .from_err()
                .map(|(_, ())| ())
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

fn topics_posts_from_list<T>(
    page: i64,
    list_key: &str,
    set_key: &'static str,
    conn: SharedConn,
) -> impl Future<Item=(Vec<T>, Vec<u32>, SharedConn), Error=Option<Vec<u32>>>
    where T: GetUserId + FromHashSetMulti {
    let start = (page as isize - 1) * 20;

    cmd("lrange")
        .arg(list_key)
        .arg(start)
        .arg(start + LIMIT - 1)
        .query_async(conn)
        .map_err(|_| None)
        .and_then(move |(conn, ids): (SharedConn, Vec<u32>)| {
            if ids.len() == 0 {
                return Either::A(fut_err(None));
            }
            Either::B(get_cache_multi_with_id(&ids, set_key, conn).map_err(|_| Some(ids)))
        })
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

    for (tid, cid, count, last_reply_time) in vec.into_iter() {
        let time = last_reply_time.timestamp_millis() - BASETIME.timestamp_millis();
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
        pipe.cmd("ZADD").arg(&format!("topic:{}:posts_reply", tid)).arg(count).arg(POST_LEX_BASE - pid).ignore()
            // set post's reply count to perm key that never expire.
            .cmd("HSET").arg(&format!("post:{}:set_perm", pid)).arg("reply_count").arg(count).ignore();
    }

    pipe.query_async(conn)
        .from_err()
        .map(|(_, ())| ())
}
