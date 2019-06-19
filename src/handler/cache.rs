use std::collections::HashMap;
use futures::{Future, future::{err as fut_err, ok as fut_ok, Either}};

use actix::prelude::*;

use actix_web::{HttpResponse, web::block};
use lazy_static::__Deref;
use r2d2_redis::{redis as redis_r, redis::{Commands, FromRedisValue, PipelineCommands, ToRedisArgs}};

use crate::model::{
    errors::ServiceError,
    category::Category,
    post::Post,
    topic::Topic,
    user::User,
    cache::{CacheQuery, FromHashSet, Parser, SortHash},
    common::{AttachUser, GetSelfId, PoolConnectionRedis, RedisPool},
    mail::Mail,
};
use crate::model::actors::{CacheService, Conn};

use redis;

const LIMIT: isize = 20;
const MAIL_LIFE: usize = 2592000;


pub struct GetCategoriesCache;

pub struct GetTopicsCache(pub Vec<u32>, pub i64);

// will panic if the query more than 20 users at a time.
pub struct GetUsersCache(pub Vec<u32>);


impl Message for GetCategoriesCache {
    type Result = Result<Vec<Category>, ServiceError>;
}

impl Message for GetTopicsCache {
    type Result = Result<(Vec<Topic>, Vec<User>), ServiceError>;
}

impl Message for GetUsersCache {
    type Result = Result<Vec<User>, ServiceError>;
}


impl Handler<GetCategoriesCache> for CacheService {
    type Result = ResponseFuture<Vec<Category>, ServiceError>;

    fn handle(&mut self, _: GetCategoriesCache, _: &mut Self::Context) -> Self::Result {
        Box::new(get_categories_cache(self.cache.as_mut().unwrap()))
    }
}

impl Handler<GetTopicsCache> for CacheService {
    type Result = ResponseFuture<(Vec<Topic>, Vec<User>), ServiceError>;

    fn handle(&mut self, msg: GetTopicsCache, _: &mut Self::Context) -> Self::Result {
        Box::new(self.cache
            .as_mut()
            .unwrap()
            .get_async_connection()
            .from_err()
            .and_then(move |conn| {
                // get topic ids from category:id:list
                let list_key = format!("category:{}:list", msg.0.first().unwrap_or(&0));
                let start = (msg.1 as isize - 1) * 20;
                redis::cmd("lrange")
                    .arg(&list_key)
                    .arg(start)
                    .arg(start + LIMIT - 1)
                    .query_async(conn)
                    .from_err()
            })
            .and_then(|(conn, tids): (Conn, Vec<u32>)| get_hmset(conn, tids, "topic"))
            .and_then(|(conn, tids, vec)| {
                let mut t = Vec::with_capacity(20);
                let mut uids = Vec::with_capacity(20);
                for v in vec.iter() {
                    if let Some(topic) = v.parse::<Topic>().ok() {
                        uids.push(topic.user_id);
                        t.push(topic);
                    }
                }
                // abort query if the topics length doesn't match the ids' length.
                if t.len() != tids.len() || t.len() == 0 {
                    return Either::A(fut_err(ServiceError::InternalServerError));
                };
                // sort and get unique users id
                uids.sort();
                uids.dedup();
                Either::B(get_users(conn, uids).and_then(move |u| Ok((t, u))))
            })
        )
    }
}

impl Handler<GetUsersCache> for CacheService {
    type Result = ResponseFuture<Vec<User>, ServiceError>;

    fn handle(&mut self, msg: GetUsersCache, _: &mut Self::Context) -> Self::Result {
        Box::new(self.cache
            .as_mut()
            .unwrap()
            .get_async_connection()
            .from_err()
            .and_then(move |conn| get_users(conn, msg.0))
        )
    }
}

pub enum UpdateCache<T> {
    Topic(Vec<T>),
    Post(Vec<T>),
    User(Vec<T>),
    Category(Vec<T>),
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
            self.cache.as_mut().unwrap(),
            vec,
            key))
    }
}

pub struct AddedTopic(pub Topic, pub u32);

impl Message for AddedTopic {
    type Result = Result<(), ServiceError>;
}

impl Handler<AddedTopic> for CacheService {
    type Result = ResponseFuture<(), ServiceError>;

    fn handle(&mut self, msg: AddedTopic, _: &mut Self::Context) -> Self::Result {
        Box::new(self.cache
            .as_mut()
            .unwrap()
            .get_async_connection()
            .from_err()
            .and_then(move |conn| {
                let t = msg.0;
                let cid = msg.1;

                let mut pipe = redis::pipe();
                pipe.atomic();
                pipe.cmd("HMSET").arg(&format!("topic:{}:set", t.get_self_id())).arg(t.sort_hash());
                pipe.cmd("HINCRBY").arg(&format!("category:{}:set", cid)).arg("topic_count").arg(1);
                pipe.cmd("lpush").arg(&format!("category:{}:list", cid)).arg(t.id);
                pipe.query_async(conn)
            })
            .from_err()
            .and_then(|(_, ())| Ok(())))
    }
}

fn get_categories_cache(
    c: &mut redis::Client
) -> impl Future<Item=Vec<Category>, Error=ServiceError> {
    c.get_async_connection()
        .from_err()
        .and_then(|conn| redis::cmd("lrange")
            .arg("category_id:meta")
            .arg(0)
            .arg(-1)
            .query_async(conn))
        .from_err()
        .and_then(|(conn, ids): (Conn, Vec<u32>)| get_hmset(conn, ids, "category"))
        .and_then(|(_, _, vec): (_, _, Vec<HashMap<String, String>>)| vec
            .iter()
            .map(|hash| hash
                .parse::<Category>())
            .collect::<Result<Vec<Category>, ServiceError>>())
}

// consume connection as users usually the last query.
pub fn get_users(
    conn: Conn,
    uids: Vec<u32>,
) -> impl Future<Item=Vec<User>, Error=ServiceError> {
    get_hmset(conn, uids, "user")
        .and_then(move |(_, uids, vec)| {
            let mut u = Vec::with_capacity(21);
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
    c: &mut redis::Client,
    vec: Vec<T>,
    key: &'static str,
) -> impl Future<Item=(), Error=ServiceError>
    where T: GetSelfId + SortHash {
    c.get_async_connection()
        .from_err()
        .and_then(move |conn| {
            let mut pipe = redis::pipe();
            pipe.atomic();
            for v in vec.iter() {
                pipe.cmd("HMSET")
                    .arg(&format!("{}:{}:set", key, v.get_self_id()))
                    .arg(v.sort_hash());
            }
            pipe.query_async(conn)
        })
        .from_err()
        .and_then(|(_, ())| Ok(()))
}

// return input vec in result
fn get_hmset<T>(
    conn: Conn,
    vec: Vec<T>,
    key: &'static str,
) -> impl Future<Item=(Conn, Vec<T>, Vec<HashMap<String, String>>), Error=ServiceError>
    where T: std::fmt::Display {
    let mut pipe = redis::pipe();
    pipe.atomic();
    for v in vec.iter() {
        pipe.cmd("HGETALL").arg(&format!("{}:{}:set", key, v));
    }
    pipe.query_async(conn)
        .from_err()
        .and_then(|(conn, hm): (Conn, Vec<HashMap<String, String>>)| Ok((conn, vec, hm)))
}

pub fn build_list(
    c: &mut redis::Client,
    vec: Vec<u32>,
    // pass lpush or rpush as cmd
    cmd: &'static str,
    key: String,
) -> impl Future<Item=(), Error=ServiceError> {
    c.get_async_connection()
        .from_err()
        .and_then(move |conn| {
            let mut pipe = redis::pipe();
            pipe.atomic();
            for v in vec.into_iter() {
                pipe.cmd(cmd).arg(&key).arg(v);
            }
            pipe.query_async(conn)
        })
        .from_err()
        .and_then(|(_, ())| Ok(()))
}



//type UpdateResult = Result<(), ServiceError>;
//
//fn added_category(c: &Vec<Category>, conn: PoolConnectionRedis) -> UpdateResult {
//    let c = c.first().ok_or(ServiceError::InternalServerError)?;
//    Ok(redis_r::pipe().atomic()
//        .rpush("category_id:meta", c.id)
//        .hset_multiple(format!("category:{}:set", c.id), &c.sort_hash())
//        .query(conn.deref())?)
//}
//
//fn got_topic_with_posts(t: Option<&Topic>, p: &Vec<Post>, conn: PoolConnectionRedis) -> UpdateResult {
//    if let Some(t) = t {
//        let _ignore: Result<usize, _> = conn.hset_multiple(format!("topic:{}:set", t.id), &t.sort_hash());
//    }
//    update_hash_set(&p, "post", conn)
//}
//
//fn added_topic(t: &Topic, c: &Category, conn: PoolConnectionRedis) -> UpdateResult {
//    Ok(redis_r::pipe().atomic()
//        .hset_multiple(format!("topic:{}:set", t.id), &t.sort_hash())
//        .hset_multiple(format!("category:{}:set", c.id), &c.sort_hash())
//        .lpush(format!("category:{}:list", c.id), t.id)
//        .query(conn.deref())?)
//}
//
//fn added_post(t: &Topic, c: &Category, p: &Post, p_old: &Option<Post>, conn: PoolConnectionRedis) -> UpdateResult {
//    let list_key = format!("category:{}:list", t.category_id);
//    Ok(match p_old {
//        Some(p_new) => redis_r::pipe().atomic()
//            .lrem(&list_key, 1, t.id)
//            .hset_multiple(format!("topic:{}:set", t.id), &t.sort_hash())
//            .hset_multiple(format!("post:{}:set", p.id), &p.sort_hash())
//            .hset_multiple(format!("post:{}:set", p_new.id), &p_new.sort_hash())
//            .hset_multiple(format!("category:{}:set", c.id), &c.sort_hash())
//            .rpush(format!("topic:{}:list", t.id), p.id)
//            .lpush(list_key, t.id)
//            .query(conn.deref())?,
//        None => redis_r::pipe().atomic()
//            .lrem(&list_key, 1, t.id)
//            .hset_multiple(format!("topic:{}:set", t.id), &t.sort_hash())
//            .hset_multiple(format!("post:{}:set", p.id), &p.sort_hash())
//            .hset_multiple(format!("category:{}:set", c.id), &c.sort_hash())
//            .rpush(format!("topic:{}:list", t.id), p.id)
//            .lpush(list_key, t.id)
//            .query(conn.deref())?
//    })
//}
//
//fn update_hash_set<T>(vec: &Vec<T>, key: &str, conn: PoolConnectionRedis) -> UpdateResult
//    where T: GetSelfId + SortHash {
//    let mut pipe = redis_r::pipe();
//    let pipe = pipe.atomic();
//
//    for v in vec {
//        pipe.hset_multiple(&format!("{}:{}:set", key, v.get_self_id()), &v.sort_hash());
//    }
//
//    Ok(pipe.query(conn.deref())?)
//}
//
pub fn clear_cache(pool: &RedisPool) -> Result<(), ServiceError> {
    let conn = pool.get().unwrap_or_else(|_|panic!("failed to connect redis pool"));
    Ok(redis_r::cmd("flushall").query(conn.deref())?)
}
//
//// helper for mail service
//pub enum MailCache {
//    AddActivation(Mail),
//    AddRecovery(Mail),
//    GotActivation(Mail),
//    GetActivation(Option<u32>),
//    RemoveActivation(u32),
//    RemoveRecovery(u32),
//}
//
//impl MailCache {
//    pub fn handler(self, pool: &RedisPool) -> impl Future<Item=(), Error=ServiceError> {
//        let pool = pool.clone();
//        block(move || match self {
//            MailCache::AddActivation(mail) => add_mail_cache(&mail, pool.get()?),
//            _ => panic!("method not allowed")
//        }).from_err()
//    }
//    pub fn from_queue(conn: &PoolConnectionRedis) -> Result<Self, ServiceError> {
//        let string = from_mail_queue(None, conn)?;
//        Ok(MailCache::GotActivation(serde_json::from_str(string.first().ok_or(ServiceError::InternalServerError)?)?))
//    }
//    pub fn get_mail_hash(&self, pool: &RedisPool) -> Result<HashMap<String, String>, ServiceError> {
//        match self {
//            MailCache::GetActivation(opt) => from_mail_hash(opt, "activation", pool.get()?),
//            _ => Err(ServiceError::BadRequest)
//        }
//    }
//    pub fn remove_queue(self, conn: &PoolConnectionRedis) -> Result<(), ServiceError> {
//        match self {
//            MailCache::GotActivation(mail) => del_mail_queue(mail.user_id, conn),
//            _ => Err(ServiceError::InternalServerError)
//        }
//    }
//}
//
//fn add_mail_cache(mail: &Mail, conn: PoolConnectionRedis) -> Result<(), ServiceError> {
//    let stringify = serde_json::to_string(mail)?;
//    let key = format!("activation:{}:set", mail.user_id);
//    Ok(redis_r::pipe().atomic()
//        .zadd("mail_queue", stringify, mail.user_id)
//        .hset_multiple(&key, &mail.sort_hash())
//        .expire(&key, MAIL_LIFE)
//        .query(conn.deref())?)
//}
//
//fn del_mail_queue(id: u32, conn: &PoolConnectionRedis) -> Result<(), ServiceError> {
//    Ok(redis_r::cmd("zrembyscore").arg("mail_queue").arg(id).arg(id).query(conn.deref())?)
//}
//
//fn from_mail_queue(opt: Option<u32>, conn: &PoolConnectionRedis) -> Result<Vec<String>, ServiceError> {
//    match opt {
//        Some(id) => Ok(redis_r::cmd("zrangebyscore").arg("mail_queue").arg(id).arg(id).query(conn.deref())?),
//        None => Ok(redis_r::cmd("zrange").arg("mail_queue").arg(0).arg(1).query(conn.deref())?)
//    }
//}
//
//fn from_mail_hash(opt: &Option<u32>, key: &str, conn: PoolConnectionRedis) -> Result<HashMap<String, String>, ServiceError> {
//    let test = opt.unwrap();
//    Ok(redis_r::cmd("hgetall").arg(format!("{}:{}:set", key, test)).query(conn.deref())?)
//}