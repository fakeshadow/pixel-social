use std::collections::HashMap;
use futures::{Future, future, future::{err as fut_err, ok as fut_ok, Either}};

use actix::prelude::*;
use chrono::{NaiveDateTime, Utc};
use redis::{Client, cmd, pipe};

use crate::model::{
    actors::{CacheService, SharedConn},
    errors::ServiceError,
    category::Category,
    post::Post,
    topic::Topic,
    user::User,
    cache::{FromHashSet, Parser, SortHash},
    common::{AttachUser, GetSelfId, GetUserId},
    mail::Mail,
};

lazy_static! {
    static ref BASETIME:NaiveDateTime = NaiveDateTime::parse_from_str("2019-06-24 2:33:33.666666", "%Y-%m-%d %H:%M:%S%.f").unwrap();
}

const LIMIT: isize = 20;
const LIMITU: usize = 20;
const MAIL_LIFE: usize = 2592000;

pub struct GetCategoriesCache;

pub enum GetTopicsCache {
    Latest(Vec<u32>, i64),
    Popular(Vec<u32>, i64),
    PopularAll(i64),
}

pub struct GetTopicCache(pub u32, pub i64);

pub struct GetPostsCache(pub Vec<u32>);

pub struct GetUsersCache(pub Vec<u32>);

pub struct AddedTopic(pub Topic);

pub struct AddedPost(pub Vec<Post>);

pub struct AddedCategory(pub Category);

pub struct RemoveCategoryCache(pub u32);

pub enum UpdateCache<T> {
    Topic(Vec<T>),
    Post(Vec<T>),
    User(Vec<T>),
    Category(Vec<T>),
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

impl Message for RemoveCategoryCache {
    type Result = Result<(), ServiceError>;
}

impl<T> Message for UpdateCache<T> {
    type Result = Result<(), ServiceError>;
}

impl<T> Handler<UpdateCache<T>> for CacheService
    where T: GetSelfId + SortHash + 'static {
    type Result = ResponseFuture<(), ServiceError>;

    fn handle(&mut self, msg: UpdateCache<T>, _: &mut Self::Context) -> Self::Result {
        let (vec, key) = match msg {
            UpdateCache::Topic(vec) => (vec, "topic"),
            UpdateCache::Post(vec) => (vec, "post"),
            UpdateCache::User(vec) => (vec, "user"),
            UpdateCache::Category(vec) => (vec, "category"),
        };
        Box::new(build_hmset(
            self.cache.as_ref().unwrap().clone(),
            vec,
            key))
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

        let mut pipe = pipe();
        pipe.atomic();
        pipe.cmd("HMSET").arg(&format!("topic:{}:set", t.self_id())).arg(t.sort_hash());
        pipe.cmd("HINCRBY").arg(&format!("category:{}:set", cid)).arg("topic_count").arg(1);
        pipe.cmd("lpush").arg(&format!("category:{}:list", cid)).arg(tid);

        // add topic to sorted set with timestamp as score. topic_id:reply_count as set.
        let time = t.last_reply_time.timestamp_millis() - BASETIME.timestamp_millis();
        let count = t.reply_count;
        pipe.cmd("ZADD").arg("all:topics:time").arg(time).arg(tid);
        pipe.cmd("ZADD").arg("all:topics:reply").arg(count).arg(tid);
        pipe.cmd("ZADD").arg(&format!("category{}:topics:time", cid)).arg(time).arg(tid);
        pipe.cmd("ZADD").arg(&format!("category{}:topics:reply", cid)).arg(count).arg(tid);

        let f = pipe
            .query_async(self.cache.as_ref().unwrap().clone())
            .from_err()
            .map(|(_, ())| ());
        Box::new(f)
    }
}

impl Handler<AddedPost> for CacheService {
    type Result = ResponseFuture<(), ServiceError>;

    fn handle(&mut self, msg: AddedPost, _: &mut Self::Context) -> Self::Result {
        let p = msg.0.first().unwrap();
        let cid = p.category_id;
        let tid = p.topic_id;
        let pid = p.id;

        let mut pipe = pipe();
        pipe.atomic();
        pipe.cmd("lrem").arg(&format!("category:{}:list", cid)).arg(1).arg(tid);
        pipe.cmd("lpush").arg(&format!("category:{}:list", cid)).arg(tid);
        pipe.cmd("rpush").arg(&format!("topic:{}:list", tid)).arg(pid);
        pipe.cmd("HMSET").arg(&format!("post:{}:set", pid)).arg(p.sort_hash());
        pipe.cmd("HINCRBY").arg(&format!("category:{}:set", cid)).arg("post_count").arg(1);
        pipe.cmd("HINCRBY").arg(&format!("topic:{}:set", tid)).arg("reply_count").arg(1);

        // add post to sorted set with timestamp as score. topic_id:reply_count as set.
        let time = p.last_reply_time.timestamp_millis() - BASETIME.timestamp_millis();
        let count = p.reply_count;
        pipe.cmd("ZADD").arg(&format!("topic{}:posts:time", tid)).arg(time).arg(pid);
        pipe.cmd("ZADD").arg(&format!("topic{}:posts:reply", tid)).arg(count).arg(pid);
        // update topic reply time and reply count
        pipe.cmd("ZADD").arg(&format!("category{}:topics:time", tid)).arg("NX").arg(time).arg(pid);
        pipe.cmd("ZINCRBY").arg(&format!("category{}:topics:reply", cid)).arg(1).arg(tid);

        if let Some(pid) = p.post_id {
            pipe.cmd("HINCRBY").arg(&format!("post:{}:set", pid)).arg("reply_count").arg(1);
            pipe.cmd("ZADD").arg(&format!("topic{}:posts:time", tid)).arg("NX").arg(time).arg(pid);
            pipe.cmd("ZINCRBY").arg(&format!("topic{}:posts:reply", tid)).arg(1).arg(pid);
        }

        let f = pipe
            .query_async(self.cache.as_ref().unwrap().clone())
            .from_err()
            .map(|(_, ())| ());
        Box::new(f)
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

impl Handler<GetTopicsCache> for CacheService {
    type Result = ResponseFuture<(Vec<Topic>, Vec<User>), ServiceError>;

    fn handle(&mut self, msg: GetTopicsCache, _: &mut Self::Context) -> Self::Result {
        match msg {
            GetTopicsCache::Latest(mut ids, page) => {
                let id = ids.pop().unwrap();

                let f =
                    topics_posts_from_list(
                        id,
                        page,
                        "category",
                        "topic",
                        self.cache.as_ref().unwrap().clone())
                        .and_then(|(t, mut uids, conn)| {
                            uids.sort();
                            uids.dedup();
                            get_users(conn, uids).map(|u| (t, u))
                        });
                Box::new(f)
            }
            GetTopicsCache::Popular(mut ids, page) => {
                let cid = ids.pop().unwrap();

                let yesterday = Utc::now().timestamp_millis() - BASETIME.timestamp_millis() - 86400000;

                let f =
                    cmd("zrevrangebyscore")
                        .arg(&format!("category{}:topics:time", cid))
                        .arg("+inf")
                        .arg(yesterday)
                        .query_async(self.cache.as_ref().unwrap().clone())
                        .from_err()
                        .and_then(move |(conn, tids): (SharedConn, Vec<u32>)| {
                            let key = format!("category{}:topics:reply", cid);
                            let mut pipe = pipe();
                            pipe.atomic();

                            for tid in tids.iter() {
                                pipe.cmd("zscore").arg(&key).arg(*tid);
                            };

                            pipe.query_async(conn)
                                .from_err()
                                .and_then(move |(conn, counts): (SharedConn, Vec<u32>)| {
                                    if counts.len() != tids.len() {
                                        return Either::A(fut_err(ServiceError::BadRequest));
                                    }
                                    let len = tids.len();

                                    let mut counts = counts.into_iter().enumerate().collect::<Vec<(usize, u32)>>();
                                    counts.sort_by(|(ia, va), (ib, vb)| vb.cmp(va));

                                    let mut vec = Vec::with_capacity(20);
                                    let mut start = ((page - 1) * 20) as usize;
                                    let finish = start + LIMITU;
                                    for i in start..finish {
                                        if i + 1 <= len {
                                            let (j, v) = counts[i];
                                            vec.push(tids[j])
                                        }
                                    }

                                    Either::B(from_hmsets(conn, vec, "topic"))
                                })
                                .and_then(|(t, uids, conn)| {
                                    get_users(conn, uids).map(|u| (t, u))
                                })
                        });


                Box::new(f)
            }
            GetTopicsCache::PopularAll(page) => {
                let id = 0;
                let f =
                    topics_posts_from_list(
                        id,
                        page,
                        "category",
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
    }
}

impl Handler<GetTopicCache> for CacheService {
    type Result = ResponseFuture<(Topic, Vec<Post>, Vec<User>), ServiceError>;

    fn handle(&mut self, msg: GetTopicCache, _: &mut Self::Context) -> Self::Result {
        let tid = msg.0;
        let page = msg.1;

        let f1 =
            get_hmset(self.cache.as_ref().unwrap().clone(), vec![tid], "topic")
                .and_then(|(_, _, vec): (_, _, Vec<HashMap<String, String>>)| {
                    let t = vec
                        .first()
                        .ok_or(ServiceError::InternalServerError)?
                        .parse::<Topic>()?;
                    Ok(t)
                });

        let f2 =
            topics_posts_from_list(
                tid,
                page,
                "topic",
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


fn topics_posts_from_list<T>(
    id: u32,
    page: i64,
    list_key: &str,
    set_key: &'static str,
    conn: SharedConn,
) -> impl Future<Item=(Vec<T>, Vec<u32>, SharedConn), Error=ServiceError>
    where T: GetUserId + FromHashSet {
    let list_key = format!("{}:{}:list", list_key, id);
    let start = (page as isize - 1) * 20;

    cmd("lrange")
        .arg(&list_key)
        .arg(start)
        .arg(start + LIMIT - 1)
        .query_async(conn)
        .from_err()
        .and_then(move |(conn, ids): (SharedConn, Vec<u32>)| from_hmsets(conn, ids, set_key))
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
            // abort query if the topics length doesn't match the ids' length.
            //  if p.len() != pids.len() || p.len() == 0
            if res.len() != ids.len() {
                return Err(ServiceError::InternalServerError);
            };
            Ok((res, uids, conn))
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
            if u.len() != uids.len() || u.len() == 0 {
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

//fn add_mail_cache(mail: &Mail, conn: PoolConnectionRedis) -> Result<(), ServiceError> {
//    let stringify = serde_json::to_string(mail)?;
//    let key = format!("activation:{}:set", mail.user_id);
//    Ok(redis_r::pipe().atomic()
//        .zadd("mail_queue", stringify, mail.user_id)
//        .hset_multiple(&key, &mail.sort_hash())
//        .expire(&key, MAIL_LIFE)
//        .query(conn.deref())?)
//}
//fn from_mail_hash(opt: &Option<u32>, key: &str, conn: PoolConnectionRedis) -> Result<HashMap<String, String>, ServiceError> {
//    let test = opt.unwrap();
//    Ok(redis_r::cmd("hgetall").arg(format!("{}:{}:set", key, test)).query(conn.deref())?)
//}

pub fn from_mail_queue(
    conn: SharedConn
) -> impl Future<Item=(SharedConn, Mail), Error=ServiceError> {
    cmd("zrange")
        .arg("mail_queue")
        .arg(0)
        .arg(1)
        .query_async(conn)
        .from_err()
        .and_then(|(conn, s): (_, Vec<String>)| s
            .first()
            .ok_or(ServiceError::MailServiceError)
            .map(|s| Ok(serde_json::from_str(s)?))
            .and_then(|r| r)
            .map(|m| (conn, m))
        )
}

pub fn remove_mail_queue(
    conn: SharedConn
) -> impl Future<Item=(), Error=ServiceError> {
    cmd("zrembyscore")
        .arg("mail_queue")
        .arg(0)
        .arg(1)
        .query_async(conn)
        .from_err()
        .map(|(_, ())| ())
}

pub fn clear_cache(redis_url: &str) -> Result<(), ServiceError> {
    let client = Client::open(redis_url).expect("failed to connect to redis server");
    let mut conn = client.get_connection().expect("failed to get redis connection");
    Ok(redis::cmd("flushall").query(&mut conn)?)
}