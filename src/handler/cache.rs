use std::{
    time::Duration,
    collections::HashMap,
};
use futures::{Future, future::{err as fut_err, Either}};

use actix::prelude::*;
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
    cache::{FromHashSet, Parser, SortHash},
    common::{GetSelfId, GetUserId},
    mail::Mail,
};

lazy_static! {
    static ref BASETIME:NaiveDateTime = NaiveDateTime::parse_from_str("2019-06-24 2:33:33.666666", "%Y-%m-%d %H:%M:%S%.f").unwrap();
}

// page offsets of list query
const LIMIT: isize = 20;
const LIMITU: usize = 20;
// list_pop update interval time gap in millis
const LIST_TIME_GAP: Duration = Duration::from_millis(10000);
// trim list_pop interval time gap
const TRIM_LIST_TIME_GAP: Duration = Duration::from_secs(3600);
// mail life is expire time of mail hash in seconds
const MAIL_LIFE: usize = 3600;

impl CacheUpdateService {
    pub fn update_list_pop(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(LIST_TIME_GAP, move |act, ctx| {
            let f =
                get_categories_cache(act.cache.as_ref().unwrap().clone())
                    .into_actor(act)
                    .map_err(|_, _, _| ())
                    .and_then(|cat, act, _| {
                        let conn = act.cache.as_ref().unwrap().clone();
                        let mut vec = Vec::new();
                        for c in cat.iter() {
                            let list_key = format!("category:{}:list_pop", c.id);
                            let time_key = format!("category:{}:topics_time", c.id);
                            let reply_key = format!("category:{}:topics_reply", c.id);

                            vec.push(update_list(time_key.as_str(), reply_key.as_str(), list_key, conn.clone()))
                        }
                        let list_key = "category:all:list_pop".to_owned();
                        let time_key = "category:all:topics_time";
                        let reply_key = "category:all:topics_reply";
                        vec.push(update_list(time_key, reply_key, list_key, conn));

                        futures::stream::futures_unordered(vec)
                            .collect()
                            .into_actor(act)
                            .map_err(|_, _, _| ())
                            .map(|_, _, _| ())
                    });

            ctx.wait(f);
        });
    }

    pub fn trim_list_pop(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(TRIM_LIST_TIME_GAP, move |act, ctx| {
            let f =
                get_categories_cache(act.cache.as_ref().unwrap().clone())
                    .into_actor(act)
                    .map_err(|_, _, _| ())
                    .and_then(|cat, act, _| {
                        let conn = act.cache.as_ref().unwrap().clone();
                        let mut vec = Vec::new();
                        for c in cat.iter() {
                            let time_key = format!("category:{}:topics_time", c.id);
                            let reply_key = format!("category:{}:topics_reply", c.id);

                            vec.push(trim_list(time_key, reply_key, conn.clone()))
                        }
                        let time_key = "category:all:topics_time".to_owned();
                        let reply_key = "category:all:topics_reply".to_owned();
                        vec.push(trim_list(time_key, reply_key, conn));

                        futures::stream::futures_unordered(vec)
                            .collect()
                            .into_actor(act)
                            .map_err(|_, _, _| ())
                            .map(|_, _, _| ())
                    });

            ctx.wait(f);
        });
    }
}


pub struct GetCategoriesCache;

pub enum GetTopicsCache {
    Latest(Vec<u32>, i64),
    Popular(Vec<u32>, i64),
    PopularAll(i64),
}

pub enum GetTopicCache {
    Old(u32, i64),
    Popular(u32, i64),
}

pub struct GetPostsCache(pub Vec<u32>);

pub struct GetUsersCache(pub Vec<u32>);

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
    type Result = Result<(Vec<Topic>, Vec<User>), ServiceError>;
}

impl Message for GetTopicCache {
    type Result = Result<(Topic, Vec<Post>, Vec<User>), ServiceError>;
}

impl Message for GetPostsCache {
    type Result = Result<(Vec<Post>, Vec<User>), ServiceError>;
}

impl Message for GetUsersCache {
    type Result = Result<Vec<User>, ServiceError>;
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
        let conn = self.cache.as_ref().unwrap().clone();

        match msg {
            UpdateCache::Topic(vec) => Box::new(build_hmset(conn, vec, "topic")),
            UpdateCache::Post(vec) => Box::new(build_hmset(conn, vec, "post")),
            UpdateCache::User(vec) => Box::new(build_hmset(conn, vec, "user")),
            UpdateCache::Category(vec) => Box::new(build_hmset(conn, vec, "category")),
        }
    }
}

impl Handler<GetCategoriesCache> for CacheService {
    type Result = ResponseFuture<Vec<Category>, ServiceError>;

    fn handle(&mut self, _: GetCategoriesCache, _: &mut Self::Context) -> Self::Result {
        Box::new(get_categories_cache(self.cache.as_mut().unwrap().clone()))
    }
}

impl Handler<GetUsersCache> for CacheService {
    type Result = ResponseFuture<Vec<User>, ServiceError>;

    fn handle(&mut self, msg: GetUsersCache, _: &mut Self::Context) -> Self::Result {
        Box::new(get_users(self.cache.as_ref().unwrap().clone(), msg.0))
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
        pipe.cmd("HMSET").arg(&format!("topic:{}:set", t.self_id())).arg(t.sort_hash()).ignore()
            .cmd("HINCRBY").arg(&format!("category:{}:set", cid)).arg("topic_count").arg(1).ignore()
            .cmd("lpush").arg(&format!("category:{}:list", cid)).arg(tid).ignore()
            .cmd("ZADD").arg("category:all:topics_time").arg(time).arg(tid).ignore()
            .cmd("ZADD").arg("category:all:topics_reply").arg(count).arg(tid).ignore()
            .cmd("ZADD").arg(&format!("category:{}:topics_time", cid)).arg(time).arg(tid).ignore()
            .cmd("ZADD").arg(&format!("category:{}:topics_reply", cid)).arg(count).arg(tid).ignore();

        Box::new(pipe
            .query_async(self.cache.as_ref().unwrap().clone())
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

        let key = format!("topic:{}:set", tid);
        let time = time.timestamp_millis() - BASETIME.timestamp_millis();

        pipe.cmd("lrem").arg(&format!("category:{}:list", cid)).arg(1).arg(tid).ignore()
            .cmd("lpush").arg(&format!("category:{}:list", cid)).arg(tid).ignore()
            .cmd("rpush").arg(&format!("topic:{}:list", tid)).arg(pid).ignore()
            .cmd("HMSET").arg(&format!("post:{}:set", pid)).arg(p.sort_hash()).ignore()
            .cmd("HINCRBY").arg(&key).arg("reply_count").arg(1).ignore()
            .cmd("HSET").arg(&key).arg("last_reply_time").arg(&time_string).ignore()
            .cmd("HINCRBY").arg(&format!("category:{}:set", cid)).arg("post_count").arg(1).ignore()

//            .cmd("ZADD").arg(&format!("topic:{}:posts_time", tid)).arg(time).arg(pid).ignore()
            .cmd("ZADD").arg(&format!("topic:{}:posts_reply", tid)).arg(count).arg(pid).ignore()
            .cmd("ZADD").arg(&format!("category:{}:topics_time", cid)).arg("XX").arg(time).arg(tid).ignore()
            .cmd("ZINCRBY").arg(&format!("category:{}:topics_reply", cid)).arg(1).arg(tid).ignore()

            .cmd("ZADD").arg("category:all:topics_time").arg("XX").arg(time).arg(tid).ignore()
            .cmd("ZINCRBY").arg("category:all:topics_reply").arg(1).arg(tid).ignore();

        if let Some(pid) = p.post_id {
            let key = format!("post:{}:set", pid);
            pipe.cmd("HSET").arg(&key).arg("last_reply_time").arg(&time_string).ignore()
                .cmd("HINCRBY").arg(&key).arg("reply_count").arg(1).ignore()
//                .cmd("ZADD").arg(&format!("topic:{}:posts_time", tid)).arg("XX").arg(time).arg(pid).ignore()
                .cmd("ZINCRBY").arg(&format!("topic:{}:posts_reply", tid)).arg(1).arg(pid).ignore();
        }

        Box::new(pipe
            .query_async(self.cache.as_ref().unwrap().clone())
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
            .query_async(self.cache.as_ref().unwrap().clone())
            .from_err()
            .map(|(_, ())| ());

        Box::new(f)
    }
}

impl Handler<GetTopicCache> for CacheService {
    type Result = ResponseFuture<(Topic, Vec<Post>, Vec<User>), ServiceError>;

    fn handle(&mut self, msg: GetTopicCache, _: &mut Self::Context) -> Self::Result {
        match msg {
            GetTopicCache::Old(tid, page) => {
                let f1 =
                    get_hmset(self.cache.as_ref().unwrap().clone(), vec![tid], "topic")
                        .and_then(|(_, _, vec): (_, _, Vec<HashMap<String, String>>)| {
                            let t = vec
                                .first()
                                .ok_or(ServiceError::InternalServerError)?
                                .parse::<Topic>()?;
                            Ok(t)
                        });

                let list_key = format!("topic:{}:list", tid);
                let f2 =
                    topics_posts_from_list(
                        page,
                        &list_key,
                        "post",
                        self.cache.as_ref().unwrap().clone());

                let f = f1
                    .join(f2)
                    .and_then(|(t, (p, mut uids, conn)): (Topic, (Vec<Post>, Vec<u32>, SharedConn))| {
                        uids.push(t.user_id);
                        uids.sort();
                        uids.dedup();
                        get_users(conn, uids).map(|u| (t, p, u))
                    });
                Box::new(f)
            }
            GetTopicCache::Popular(tid, page) => {
                let f1 =
                    get_hmset(self.cache.as_ref().unwrap().clone(), vec![tid], "topic")
                        .and_then(|(_, _, vec): (_, _, Vec<HashMap<String, String>>)| {
                            let t = vec
                                .first()
                                .ok_or(ServiceError::InternalServerError)?
                                .parse::<Topic>()?;
                            Ok(t)
                        });

                let count_key = format!("topic:{}:posts_reply", tid);
                let f2 = cmd("zrevrangebyscore")
                    .arg(&count_key)
                    .arg("+inf")
                    .arg("-inf")
                    .arg("WITHSCORES")
                    .query_async(self.cache.as_ref().unwrap().clone())
                    .from_err()
                    .and_then(move |(conn, mut pids): (SharedConn, Vec<(u32, u32)>)| {
                        pids.sort_by(|(ia, sa), (ib, sb)| {
                            if sa == sb {
                                ia.cmp(ib)
                            } else {
                                sb.cmp(sa)
                            }
                        });
                        let len = pids.len();
                        let mut vec = Vec::with_capacity(20);
                        let start = ((page - 1) * 20) as usize;
                        for i in start..start + LIMITU {
                            if i + 1 <= len {
                                vec.push(pids[i].0)
                            }
                        }
                        Ok((conn, vec))
                    })
                    .and_then(move |(conn, vec)| from_hmsets(conn, vec, "post"));

                let f = f1
                    .join(f2)
                    .and_then(|(t, (p, mut uids, conn)): (Topic, (Vec<Post>, Vec<u32>, SharedConn))| {
                        uids.push(t.user_id);
                        uids.sort();
                        uids.dedup();
                        get_users(conn, uids).map(|u| (t, p, u))
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
            from_hmsets(self.cache.as_ref().unwrap().clone(), msg.0, "post")
                .and_then(|(p, mut uids, conn)| {
                    uids.sort();
                    uids.dedup();
                    get_users(conn, uids).map(|u| (p, u))
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

        Box::new(pipe.query_async(self.cache.as_ref().unwrap().clone())
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
            .query_async(self.cache.as_ref().unwrap().clone())
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
            .query_async(self.cache.as_ref().unwrap().clone())
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
            .query_async(self.cache.as_ref().unwrap().clone())
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
    type Result = ResponseFuture<(Vec<Topic>, Vec<User>), ServiceError>;

    fn handle(&mut self, msg: GetTopicsCache, _: &mut Self::Context) -> Self::Result {
        let (key, page) = match msg {
            GetTopicsCache::Popular(ids, page) =>
                (format!("category:{}:list_pop", ids.first().unwrap()), page),
            GetTopicsCache::PopularAll(page) =>
                ("category:all:list_pop".to_owned(), page),
            GetTopicsCache::Latest(ids, page) =>
                (format!("category:{}:list", ids.first().unwrap()), page),
        };

        let f =
            topics_posts_from_list(
                page,
                key.as_str(),
                "topic",
                self.cache.as_ref().unwrap().clone())
                .and_then(|(t, mut uids, conn)| {
                    uids.sort();
                    uids.dedup();
                    get_users(conn, uids).map(|u| (t, u))
                });

        Box::new(f)
    }
}

fn update_list(
    time_key: &str,
    reply_key: &str,
    list_key: String,
    conn: SharedConn,
) -> impl Future<Item=(), Error=ServiceError> {
    let yesterday = Utc::now().timestamp_millis() - BASETIME.timestamp_millis() - 86400000;

    let mut pip = pipe();
    pip.atomic();

    pip.cmd("zrevrangebyscore")
        .arg(time_key)
        .arg("+inf")
        .arg(yesterday)
        .cmd("ZREVRANGEBYSCORE")
        .arg(reply_key)
        .arg("+inf")
        .arg("-inf")
        .arg("WITHSCORES");

    pip.query_async(conn)
        .from_err()
        .and_then(move |(conn, (tids, counts)): (_, (Vec<u32>, Vec<(u32, u32)>))| {
            let len = counts.len();
            let mut temp: Vec<(u32, u32)> = Vec::with_capacity(len);
            let mut vec = Vec::with_capacity(len);

            // sort two ranged scores with the last_reply_time desc and reply_count desc.
            for i in 0..tids.len() {
                for (tid, count) in counts.iter() {
                    if &tids[i] == tid {
                        let l = temp.len();
                        if l == 0 {
                            temp.push((*tid, *count));
                            vec.push(*tid);
                        } else {
                            for k in 0..l {
                                if count > &temp[k].1 {
                                    let k = if k > 1 { k - 1 } else { 0 };
                                    temp.insert(k, (*tid, *count));
                                    vec.insert(k, *tid);
                                    break;
                                } else {
                                    temp.push((*tid, *count));
                                    vec.push(*tid);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            let mut pipe = pipe();
            pipe.atomic();
            pipe.cmd("del").arg(&list_key).ignore()
                .cmd("rpush").arg(&list_key).arg(vec).ignore();
            pipe.query_async(conn)
                .from_err()
                .map(|(_, ())| ())
        })
}

fn trim_list(
    time_key: String,
    reply_key: String,
    conn: SharedConn,
) -> impl Future<Item=(), Error=ServiceError> {
    let yesterday = Utc::now().timestamp_millis() - BASETIME.timestamp_millis() - 86400000;

    cmd("zrevrangebyscore")
        .arg(&time_key)
        .arg(yesterday)
        .arg("-inf")
        .query_async(conn)
        .from_err()
        .and_then(move |(conn, tids): (_, Vec<u32>)| {
            let mut pipe = pipe();

            for tid in tids.into_iter() {
                pipe.cmd("zrem").arg(&reply_key).arg(tid).ignore();
            }
            pipe.cmd("ZREMRANGEBYSCORE").arg(&time_key).arg(yesterday).arg("-inf").ignore();
            pipe.query_async(conn)
                .from_err()
                .map(|(_, ())| ())
        })
}

fn topics_posts_from_list<T>(
    page: i64,
    list_key: &str,
    set_key: &'static str,
    conn: SharedConn,
) -> impl Future<Item=(Vec<T>, Vec<u32>, SharedConn), Error=ServiceError>
    where T: GetUserId + FromHashSet {
    let start = (page as isize - 1) * 20;

    cmd("lrange")
        .arg(list_key)
        .arg(start)
        .arg(start + LIMIT - 1)
        .query_async(conn)
        .from_err()
        .and_then(move |(conn, ids): (SharedConn, Vec<u32>)| {
            if ids.len() == 0 {
                return Either::A(fut_err(ServiceError::NoCache));
            }
            Either::B(from_hmsets(conn, ids, set_key))
        })
}

fn get_categories_cache(
    conn: SharedConn,
) -> impl Future<Item=Vec<Category>, Error=ServiceError> {
    cmd("lrange")
        .arg("category_id:meta")
        .arg(0)
        .arg(-1)
        .query_async(conn)
        .from_err()
        .and_then(|(conn, vec): (_, Vec<u32>)| get_hmset(conn, vec, "category"))
        .and_then(|(_, _, vec): (_, _, Vec<HashMap<String, String>>)| vec
            .iter()
            .map(|hash| hash
                .parse::<Category>())
            .collect::<Result<Vec<Category>, ServiceError>>())
}

// consume connection as users usually the last query.
pub fn get_users(
    conn: SharedConn,
    uids: Vec<u32>,
) -> impl Future<Item=Vec<User>, Error=ServiceError> {
    get_hmset(conn, uids, "user")
        .and_then(move |(_, uids, vec)| {
            let mut u = Vec::new();
            // collect users from hash map
            for v in vec.iter() {
                if let Some(user) = v.parse::<User>().ok() {
                    u.push(user);
                }
            };
            // abort
            if u.len() != uids.len() {
                return Err(ServiceError::InternalServerError);
            };
            Ok(u)
        })
}

// helper functions
pub fn build_hmset<T>(
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

fn from_hmsets<T>(
    conn: SharedConn,
    ids: Vec<u32>,
    set_key: &'static str,
) -> impl Future<Item=(Vec<T>, Vec<u32>, SharedConn), Error=ServiceError>
    where T: GetUserId + FromHashSet {
    get_hmset(conn, ids, set_key)
        .and_then(|(conn, ids, vec): (SharedConn, Vec<u32>, Vec<HashMap<String, String>>)| {
            let mut res: Vec<T> = Vec::with_capacity(20);
            let mut uids: Vec<u32> = Vec::with_capacity(21);
            for v in vec.iter() {
                if let Some(t) = v.parse::<T>().ok() {
                    uids.push(t.get_user_id());
                    res.push(t);
                }
            }
            if res.len() != ids.len() {
                return Err(ServiceError::InternalServerError);
            };
            Ok((res, uids, conn))
        })
}

fn get_hmset<T>(
    conn: SharedConn,
    vec: Vec<T>,
    key: &'static str,
) -> impl Future<Item=(SharedConn, Vec<T>, Vec<HashMap<String, String>>), Error=ServiceError>
    where T: std::fmt::Display {
    let mut pipe = pipe();
    pipe.atomic();
    for v in vec.iter() {
        pipe.cmd("HGETALL").arg(&format!("{}:{}:set", key, v));
    }
    pipe.query_async(conn)
        .from_err()
        .map(|(conn, hm)| (conn, vec, hm))
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

pub fn build_category_set(
    vec: Vec<(u32, u32, i32, NaiveDateTime)>,
    conn: SharedConn,
) -> impl Future<Item=(), Error=ServiceError> {
    let mut pipe = pipe();
    pipe.atomic();

    for (tid, cid, count, last_reply_time) in vec.into_iter() {
        let time = last_reply_time.timestamp_millis() - BASETIME.timestamp_millis();
        pipe.cmd("ZADD").arg("category:all:topics_time").arg(time).arg(tid).ignore()
            .cmd("ZADD").arg("category:all:topics_reply").arg(count).arg(tid).ignore()
            .cmd("ZADD").arg(&format!("category:{}:topics_time", cid)).arg(time).arg(tid).ignore()
            .cmd("ZADD").arg(&format!("category:{}:topics_reply", cid)).arg(count).arg(tid).ignore();
    }

    pipe.query_async(conn)
        .from_err()
        .map(|(_, ())| ())
}

pub fn build_topic_set(
    vec: Vec<(u32, u32, i32, NaiveDateTime)>,
    conn: SharedConn,
) -> impl Future<Item=(), Error=ServiceError> {
    let mut pipe = pipe();
    pipe.atomic();

    for (tid, pid, count, _) in vec.into_iter() {
        pipe.cmd("ZADD").arg(&format!("topic:{}:posts_reply", tid)).arg(count).arg(pid).ignore();
    }

    pipe.query_async(conn)
        .from_err()
        .map(|(_, ())| ())
}
