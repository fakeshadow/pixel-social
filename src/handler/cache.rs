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
    common::{AttachUser, get_unique_id, GetSelfId, GetUserId, PoolConnectionRedis, RedisPool},
    mail::Mail,
};
use crate::model::db::{RedisConnection, Conn};

use redis;

const LIMIT: isize = 20;
const MAIL_LIFE: usize = 2592000;


pub struct GetCategoriesCache;

impl Message for GetCategoriesCache {
    type Result = Result<Vec<Category>, ServiceError>;
}

impl Handler<GetCategoriesCache> for RedisConnection {
    type Result = ResponseFuture<Vec<Category>, ServiceError>;

    fn handle(&mut self, _: GetCategoriesCache, _: &mut Self::Context) -> Self::Result {
        Box::new(self.cache
            .as_mut()
            .unwrap()
            .get_async_connection()
            .map_err(|_| ServiceError::BadRequest)
            .and_then(|conn| redis::cmd("lrange")
                .arg("category_id:meta")
                .arg(0)
                .arg(-1)
                .query_async(conn)
                .map_err(|_| ServiceError::BadRequest)
                .and_then(|(conn, ids): (Conn, Vec<u32>)| {
                    let mut pipe = redis::pipe();
                    for id in ids {
                        pipe.cmd("HGETALL").arg(format!("category:{}:set", id));
                    }
                    pipe.query_async(conn)
                        .map_err(|_| ServiceError::BadRequest)
                        .and_then(|(conn, vec): (Conn, Vec<HashMap<String, String>>)| {
                            let r: Result<Vec<Category>, ServiceError> =
                                vec.iter()
                                    .map(|hash| hash.parse::<Category>())
                                    .collect();
                            r
                        })
                })
            ))
    }
}

pub struct GetTopicsCache(pub Vec<u32>, pub i64);

impl Message for GetTopicsCache {
    type Result = Result<(Vec<Topic>, Vec<User>), ServiceError>;
}

impl Handler<GetTopicsCache> for RedisConnection {
    type Result = ResponseFuture<(Vec<Topic>, Vec<User>), ServiceError>;

    fn handle(&mut self, msg: GetTopicsCache, _: &mut Self::Context) -> Self::Result {
        Box::new(self.cache
            .as_mut()
            .unwrap()
            .get_async_connection()
            .map_err(|_| ServiceError::BadRequest)
            .and_then(move |conn| {
                // get topic ids from category:id:list
                let list_key = format!("category:{}:list", msg.0.first().unwrap_or(&0));
                let start = (msg.1 as isize - 1) * 20;
                redis::cmd("lrange")
                    .arg(&list_key)
                    .arg(start)
                    .arg(start + LIMIT - 1)
                    .query_async(conn)
                    .map_err(|_| ServiceError::BadRequest)
            })
            .and_then(|(conn, tids): (Conn, Vec<u32>)| {
                // pipeline query with th topic ids to get hash map.
                let mut pipe = redis::pipe();
                for id in tids.iter() {
                    pipe.cmd("HGETALL").arg(format!("topic:{}:set", id));
                }
                pipe.query_async(conn)
                    .map_err(|_| ServiceError::BadRequest)
                    .and_then(move |(conn, vec): (Conn, Vec<HashMap<String, String>>)| {
                        // collect topics and topic user_ids from hash map
                        let mut t = Vec::with_capacity(20);
                        let mut uids = Vec::with_capacity(20);
                        for v in vec.iter() {
                            let r = v.parse::<Topic>();
                            if let Some(topic) = r.ok() {
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
                        let mut pipe = redis::pipe();
                        for id in uids.iter() {
                            pipe.cmd("HGETALL").arg(format!("user:{}:set", id));
                        }
                        // pipeline query with the users id to get hash map
                        Either::B(pipe.query_async(conn)
                            .map_err(|_| ServiceError::BadRequest)
                            .and_then(move |(conn, vec): (Conn, Vec<HashMap<String, String>>)| {
                                let mut u = Vec::with_capacity(20);

                                // collect users from hash map
                                for v in vec.iter() {
                                    let r = v.parse::<User>();
                                    if let Some(user) = r.ok() {
                                        u.push(user);
                                    }
                                };

                                // abort
                                if u.len() != uids.len() || u.len() == 0 {
                                    return Err(ServiceError::InternalServerError);
                                };
                                Ok((t, u))
                            }))
                    })
            })
        )
    }
}

pub struct UpdateCache<T>(pub Vec<T>, pub String);

impl<T> Message for UpdateCache<T> {
    type Result = Result<(), ServiceError>;
}

impl<T> Handler<UpdateCache<T>> for RedisConnection
    where T: GetSelfId + SortHash + std::fmt::Debug + 'static {
    type Result = ResponseFuture<(), ServiceError>;

    fn handle(&mut self, msg: UpdateCache<T>, _: &mut Self::Context) -> Self::Result {
        Box::new(self.cache
            .as_mut()
            .unwrap()
            .get_async_connection()
            .map_err(|_| ServiceError::BadRequest)
            .and_then(move |conn| {
                let key = msg.1;
                let vec = msg.0;
                let mut pipe = redis::pipe();
                for v in vec.iter() {
                    pipe.cmd("HMSET").arg(&format!("{}:{}:set", key, v.get_self_id())).arg(v.sort_hash());
                }
                pipe.query_async(conn)
                    .map_err(|_| ServiceError::BadRequest)
                    .and_then(move |(conn, _): (Conn, ())| Ok(()))
            }))
    }
}

impl CacheQuery {
    pub fn into_user(self, pool: &RedisPool) -> impl Future<Item=User, Error=ServiceError> {
        let pool = pool.clone();
        block(move || match self {
            CacheQuery::GetUser(id) => get_user(id, &pool.get()?),
            _ => panic!("method not allowed")
        }).from_err()
    }

    pub fn into_topics(self, pool: RedisPool) -> impl Future<Item=Vec<Topic>, Error=ServiceError> {
        block(move || match self {
            CacheQuery::GetTopics(ids, page) => get_topics(&ids, &page, &pool.get()?),
            _ => panic!("method not allowed")
        }).from_err()
    }

    pub fn into_topic_with_post(self, pool: RedisPool) -> impl Future<Item=(Option<Topic>, Vec<Post>), Error=ServiceError> {
        block(move || match self {
            CacheQuery::GetTopic(id, page) => get_topic(id, page, &pool.get()?),
            _ => panic!("method not allowed")
        }).from_err()
    }

    pub fn into_categories(self, pool: &RedisPool) -> impl Future<Item=Vec<Category>, Error=ServiceError> {
        let pool = pool.clone();
        block(move || match self {
            CacheQuery::GetAllCategories => get_categories(&pool.get()?),
            _ => panic!("method not allowed")
        }).from_err()
    }

    pub fn into_post(self, pool: &RedisPool) -> impl Future<Item=Vec<Post>, Error=ServiceError> {
        let pool = pool.clone();
        block(move || match self {
            CacheQuery::GetPost(id) => get_post(id, &pool.get()?),
            _ => panic!("method not allowed")
        }).from_err()
    }
}

fn get_user(id: u32, conn: &PoolConnectionRedis) -> Result<User, ServiceError> {
    let hash = get_hash_set(&vec![id], "user", conn)?.pop().ok_or(ServiceError::InternalServerError)?;
    hash.parse::<User>()
}

fn get_categories(conn: &PoolConnectionRedis) -> Result<Vec<Category>, ServiceError> {
    let ids = get_meta::<u32>("category_id", &conn)?;
    from_hash_set::<Category>(&ids, "category", &conn)
}

fn get_topics(ids: &Vec<u32>, page: &i64, conn: &PoolConnectionRedis) -> Result<Vec<Topic>, ServiceError> {
    let list_key = format!("category:{}:list", ids.first().unwrap_or(&0));
    let start = (*page as isize - 1) * 20;
    let ids: Vec<u32> = redis_r::cmd("lrange").arg(&list_key).arg(start).arg(start + LIMIT - 1).query(conn.deref())?;
    from_hash_set::<Topic>(&ids, "topic", &conn)
}

fn get_topic(id: u32, page: i64, conn: &PoolConnectionRedis) -> Result<(Option<Topic>, Vec<Post>), ServiceError> {
    let t: Option<Topic> = if page == 1 {
        from_hash_set::<Topic>(&vec![id.clone()], "topic", conn)?.pop()
    } else { None };

    let list_key = format!("topic:{}:list", id);
    let start = (page as isize - 1) * 20;
    let p_ids: Vec<u32> = redis_r::cmd("lrange").arg(&list_key).arg(start).arg(start + LIMIT - 1).query(conn.deref())?;

    // ToDo: Handle case when posts are empty
    let p = from_hash_set::<Post>(&p_ids, "post", conn)?;
    Ok((t, p))
}

fn get_post(id: u32, conn: &PoolConnectionRedis) -> Result<Vec<Post>, ServiceError> {
    from_hash_set::<Post>(&vec![id], "post", conn)
}

pub fn get_unique_users_cache<T>(vec: &Vec<T>, opt: Option<u32>, pool: RedisPool)
                                 -> impl Future<Item=Vec<User>, Error=ServiceError>
    where T: GetUserId {
    let ids = get_unique_id(vec, opt);
    block(move || Ok(from_hash_set::<User>(&ids, "user", &pool.get()?)?)).from_err()
}

/// use Parser and FromHashSet traits to convert HashMap into struct.
fn from_hash_set<T>(ids: &Vec<u32>, key: &str, conn: &PoolConnectionRedis) -> Result<Vec<T>, ServiceError>
    where T: FromHashSet {
    let vec = get_hash_set(ids, key, &conn)?;
    if vec.len() != ids.len() { return Err(ServiceError::InternalServerError); };
    vec.iter().map(|hash| hash.parse::<T>()).collect()
}

fn get_hash_set(ids: &Vec<u32>, key: &str, conn: &PoolConnectionRedis) -> Result<Vec<HashMap<String, String>>, ServiceError> {
    let mut pipe = redis_r::pipe();
    let pipe = pipe.atomic();

    for id in ids {
        pipe.hgetall(format!("{}:{}:set", key, id));
    }
    Ok(pipe.query(conn.deref())?)
}

pub enum UpdateCacheAsync {
    GotTopics(Vec<Topic>),
    GotPost(Post),
    GotUser(User),
    GotCategories(Vec<Category>),
    GotTopicWithPosts(Option<Topic>, Vec<Post>),
    AddedTopic(Category, Topic),
    AddedPost(Category, Topic, Option<Post>, Post),
    AddedCategory(Vec<Category>),
    DeleteCategory(u32),
}

impl UpdateCacheAsync {
    pub fn handler(self, pool: &RedisPool) -> impl Future<Item=(), Error=ServiceError> {
        let pool = pool.clone();
        block(move || match self {
            UpdateCacheAsync::GotTopics(t) => update_hash_set(&t, "topic", pool.get()?),
            UpdateCacheAsync::GotCategories(c) => update_hash_set(&c, "category", pool.get()?),
            UpdateCacheAsync::GotTopicWithPosts(t, p) => got_topic_with_posts(t.as_ref(), &p, pool.get()?),
            UpdateCacheAsync::AddedPost(c, t, p, p_new) => added_post(&t, &c, &p_new, &p, pool.get()?),
            UpdateCacheAsync::AddedTopic(c, t) => added_topic(&t, &c, pool.get()?),
            UpdateCacheAsync::AddedCategory(c) => added_category(&c, pool.get()?),
            UpdateCacheAsync::GotPost(p) => Ok(pool.get()?.hset_multiple(&format!("post:{}:set", p.id), &p.sort_hash())?),
            UpdateCacheAsync::GotUser(u) => Ok(pool.get()?.hset_multiple(&format!("user:{}:set", u.id), &u.sort_hash())?),
            // ToDo: migrate post and topic when deleting category
            UpdateCacheAsync::DeleteCategory(id) => Ok(redis_r::pipe()
                .lrem("category_id:meta", 1, id)
                .del(format!("{}:{}:set", "category", id))
                .query(pool.get()?.deref())?),
        }).from_err()
    }
}

type UpdateResult = Result<(), ServiceError>;

fn added_category(c: &Vec<Category>, conn: PoolConnectionRedis) -> UpdateResult {
    let c = c.first().ok_or(ServiceError::InternalServerError)?;
    Ok(redis_r::pipe().atomic()
        .rpush("category_id:meta", c.id)
        .hset_multiple(format!("category:{}:set", c.id), &c.sort_hash())
        .query(conn.deref())?)
}

fn got_topic_with_posts(t: Option<&Topic>, p: &Vec<Post>, conn: PoolConnectionRedis) -> UpdateResult {
    if let Some(t) = t {
        let _ignore: Result<usize, _> = conn.hset_multiple(format!("topic:{}:set", t.id), &t.sort_hash());
    }
    update_hash_set(&p, "post", conn)
}

fn added_topic(t: &Topic, c: &Category, conn: PoolConnectionRedis) -> UpdateResult {
    Ok(redis_r::pipe().atomic()
        .hset_multiple(format!("topic:{}:set", t.id), &t.sort_hash())
        .hset_multiple(format!("category:{}:set", c.id), &c.sort_hash())
        .lpush(format!("category:{}:list", c.id), t.id)
        .query(conn.deref())?)
}

fn added_post(t: &Topic, c: &Category, p: &Post, p_old: &Option<Post>, conn: PoolConnectionRedis) -> UpdateResult {
    let list_key = format!("category:{}:list", t.category_id);
    Ok(match p_old {
        Some(p_new) => redis_r::pipe().atomic()
            .lrem(&list_key, 1, t.id)
            .hset_multiple(format!("topic:{}:set", t.id), &t.sort_hash())
            .hset_multiple(format!("post:{}:set", p.id), &p.sort_hash())
            .hset_multiple(format!("post:{}:set", p_new.id), &p_new.sort_hash())
            .hset_multiple(format!("category:{}:set", c.id), &c.sort_hash())
            .rpush(format!("topic:{}:list", t.id), p.id)
            .lpush(list_key, t.id)
            .query(conn.deref())?,
        None => redis_r::pipe().atomic()
            .lrem(&list_key, 1, t.id)
            .hset_multiple(format!("topic:{}:set", t.id), &t.sort_hash())
            .hset_multiple(format!("post:{}:set", p.id), &p.sort_hash())
            .hset_multiple(format!("category:{}:set", c.id), &c.sort_hash())
            .rpush(format!("topic:{}:list", t.id), p.id)
            .lpush(list_key, t.id)
            .query(conn.deref())?
    })
}

fn update_hash_set<T>(vec: &Vec<T>, key: &str, conn: PoolConnectionRedis) -> UpdateResult
    where T: GetSelfId + SortHash {
    let mut pipe = redis_r::pipe();
    let pipe = pipe.atomic();

    for v in vec {
        pipe.hset_multiple(&format!("{}:{}:set", key, v.get_self_id()), &v.sort_hash());
    }

    Ok(pipe.query(conn.deref())?)
}

/// Category meta store all active category ids.
pub fn update_meta<T>(ids: Vec<T>, foreign_key: &str, conn: &PoolConnectionRedis) -> UpdateResult
    where T: ToRedisArgs {
    let key = format!("{}:meta", foreign_key);
    ids.into_iter().map(|id| Ok(conn.rpush(&key, id)?)).collect()
}

fn get_meta<T>(key: &str, conn: &PoolConnectionRedis) -> Result<Vec<T>, ServiceError>
    where T: FromRedisValue {
    Ok(conn.lrange(format!("{}:meta", key), 0, -1)?)
}

pub fn build_hash_set<T>(vec: &Vec<T>, key: &str, conn: &PoolConnectionRedis) -> UpdateResult
    where T: GetSelfId + SortHash {
    vec.iter().map(|v| Ok(conn.hset_multiple(format!("{}:{}:set", key, v.get_self_id()), &v.sort_hash())?)).collect()
}

pub fn build_list(ids: Vec<u32>, foreign_key: &str, conn: &PoolConnectionRedis) -> UpdateResult {
    let key = format!("{}:list", foreign_key);
    ids.into_iter().map(|id| Ok(conn.rpush(&key, id)?)).collect()
}

pub fn clear_cache(pool: &RedisPool) -> Result<(), ServiceError> {
    let conn = pool.get()?;
    Ok(redis_r::cmd("flushall").query(conn.deref())?)
}

// helper for mail service
pub enum MailCache {
    AddActivation(Mail),
    AddRecovery(Mail),
    GotActivation(Mail),
    GetActivation(Option<u32>),
    RemoveActivation(u32),
    RemoveRecovery(u32),
}

impl MailCache {
    pub fn handler(self, pool: &RedisPool) -> impl Future<Item=(), Error=ServiceError> {
        let pool = pool.clone();
        block(move || match self {
            MailCache::AddActivation(mail) => add_mail_cache(&mail, pool.get()?),
            _ => panic!("method not allowed")
        }).from_err()
    }
    pub fn from_queue(conn: &PoolConnectionRedis) -> Result<Self, ServiceError> {
        let string = from_mail_queue(None, conn)?;
        Ok(MailCache::GotActivation(serde_json::from_str(string.first().ok_or(ServiceError::InternalServerError)?)?))
    }
    pub fn get_mail_hash(&self, pool: &RedisPool) -> Result<HashMap<String, String>, ServiceError> {
        match self {
            MailCache::GetActivation(opt) => from_mail_hash(opt, "activation", pool.get()?),
            _ => Err(ServiceError::BadRequest)
        }
    }
    pub fn remove_queue(self, conn: &PoolConnectionRedis) -> Result<(), ServiceError> {
        match self {
            MailCache::GotActivation(mail) => del_mail_queue(mail.user_id, conn),
            _ => Err(ServiceError::InternalServerError)
        }
    }
}

fn add_mail_cache(mail: &Mail, conn: PoolConnectionRedis) -> Result<(), ServiceError> {
    let stringify = serde_json::to_string(mail)?;
    let key = format!("activation:{}:set", mail.user_id);
    Ok(redis_r::pipe().atomic()
        .zadd("mail_queue", stringify, mail.user_id)
        .hset_multiple(&key, &mail.sort_hash())
        .expire(&key, MAIL_LIFE)
        .query(conn.deref())?)
}

fn del_mail_queue(id: u32, conn: &PoolConnectionRedis) -> Result<(), ServiceError> {
    Ok(redis_r::cmd("zrembyscore").arg("mail_queue").arg(id).arg(id).query(conn.deref())?)
}

fn from_mail_queue(opt: Option<u32>, conn: &PoolConnectionRedis) -> Result<Vec<String>, ServiceError> {
    match opt {
        Some(id) => Ok(redis_r::cmd("zrangebyscore").arg("mail_queue").arg(id).arg(id).query(conn.deref())?),
        None => Ok(redis_r::cmd("zrange").arg("mail_queue").arg(0).arg(1).query(conn.deref())?)
    }
}

fn from_mail_hash(opt: &Option<u32>, key: &str, conn: PoolConnectionRedis) -> Result<HashMap<String, String>, ServiceError> {
    let test = opt.unwrap();
    Ok(redis_r::cmd("hgetall").arg(format!("{}:{}:set", key, test)).query(conn.deref())?)
}