use std::collections::HashMap;
use futures::Future;

use actix_web::{HttpResponse, web::{Data, block}};
use lazy_static::__Deref;
use r2d2_redis::{redis, redis::{Commands, FromRedisValue, PipelineCommands, ToRedisArgs}};

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

const LIMIT: isize = 20;
const LIMIT_U: usize = 20;
const MAIL_LIFE: usize = 2592000;

impl CacheQuery {
    pub fn into_user(self, pool: &RedisPool) -> impl Future<Item=User, Error=ServiceError> {
        let pool = pool.clone();
        block(move || match self {
            CacheQuery::GetUser(id) => get_user(id, &pool.get()?),
            _ => panic!("method not allowed")
        }).from_err()
    }

    pub fn into_topics(self, pool: &RedisPool) -> impl Future<Item=Vec<Topic>, Error=ServiceError> {
        let pool = pool.clone();
        block(move || match self {
            CacheQuery::GetTopics(ids, page) => get_topics(&ids, &page, &pool.get()?),
            _ => panic!("method not allowed")
        }).from_err()
    }

    pub fn into_topic_with_post(self, pool: &RedisPool) -> impl Future<Item=(Option<Topic>, Vec<Post>), Error=ServiceError> {
        let pool = pool.clone();
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
    let hash = get_hash_set(&vec![id], "user", conn)?.pop().ok_or(ServiceError::NoCacheFound)?;
    hash.parse::<User>()
}

fn get_categories(conn: &PoolConnectionRedis) -> Result<Vec<Category>, ServiceError> {
    // ToDo: need further look into the logic
    let mut categories_total = get_meta::<u32>("category_id", &conn)?;
    let total = categories_total.len();

    let mut categories_hash_vec = Vec::with_capacity(total);
    while categories_total.len() > LIMIT_U {
        let index = categories_total.len() - LIMIT_U;
        let slice = categories_total.drain(index..).collect();
        for t in get_hash_set(&slice, "category", &conn)?.into_iter() {
            if !t.is_empty() { categories_hash_vec.push(t) }
        }
    }
    for t in get_hash_set(&categories_total, "category", &conn)?.into_iter() {
        if !t.is_empty() { categories_hash_vec.push(t) }
    }
    if categories_hash_vec.len() != total { return Err(ServiceError::NoCacheFound); }
    categories_hash_vec.iter().map(|hash| hash.parse()).collect()
}

fn get_topics(ids: &Vec<u32>, page: &i64, conn: &PoolConnectionRedis) -> Result<Vec<Topic>, ServiceError> {
    let list_key = format!("category:{}:list", ids.first().unwrap_or(&0));
    let start = (*page as isize - 1) * 20;

    let topic_id: Vec<u32> = conn.lrange(&list_key, start, start + LIMIT - 1)?;
    from_hash_set::<Topic>(&topic_id, "topic", &conn)
}

fn get_topic(id: u32, page: i64, conn: &PoolConnectionRedis) -> Result<(Option<Topic>, Vec<Post>), ServiceError> {
    let topic: Option<Topic> = if page == 1 {
        from_hash_set::<Topic>(&vec![id.clone()], "topic", conn)?.pop()
    } else { None };

    let list_key = format!("topic:{}:list", id);
    let start = (page as isize - 1) * 20;
    let post_id: Vec<u32> = redis::cmd("lrange").arg(&list_key).arg(start).arg(start + LIMIT - 1).query(conn.deref())?;

    // ToDo: Handle case when posts are empty
    let posts = from_hash_set::<Post>(&post_id, "post", conn)?;
    Ok((topic, posts))
}

fn get_post(id: u32, conn: &PoolConnectionRedis) -> Result<Vec<Post>, ServiceError> {
    from_hash_set::<Post>(&vec![id], "post", conn)
}

pub fn get_unique_users_cache<T>(vec: &Vec<T>, opt: Option<u32>, pool: &RedisPool)
                                 -> impl Future<Item=Vec<User>, Error=ServiceError>
    where T: GetUserId {
    let ids = get_unique_id(vec, opt);
    let pool = pool.clone();
    block(move || Ok(from_hash_set::<User>(&ids, "user", &pool.get()?)?)).from_err()
}


/// use Parser and FromHashSet traits to convert HashMap into struct.
fn from_hash_set<T>(ids: &Vec<u32>, key: &str, conn: &PoolConnectionRedis) -> Result<Vec<T>, ServiceError>
    where T: FromHashSet {
    let vec = get_hash_set(ids, key, &conn)?;
    if vec.len() != ids.len() { return Err(ServiceError::NoCacheFound); };
    vec.iter().map(|hash| hash.parse::<T>()).collect()
}

// ToDo: make a more compat macro to handle pipeline
fn get_hash_set(ids: &Vec<u32>, key: &str, conn: &PoolConnectionRedis) -> Result<Vec<HashMap<String, String>>, ServiceError> {
    macro_rules! pipeline {
        ( $ y: expr; $( $ x: expr),*) =>(redis::pipe().atomic() $ (.hgetall(format!("{}:{}:set", $ y, $ x)))*);
    }
    if ids.len() == 1 {
        Ok(pipeline![key; ids[0]].query(conn.deref())?)
    } else if ids.len() == 2 {
        Ok(pipeline![key; ids[0], ids[1]].query(conn.deref())?)
    } else if ids.len() == 3 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2]).query(conn.deref())?)
    } else if ids.len() == 4 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2], ids[3]).query(conn.deref())?)
    } else if ids.len() == 5 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2], ids[3], ids[4]).query(conn.deref())?)
    } else if ids.len() == 6 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2], ids[3], ids[4],ids[5]).query(conn.deref())?)
    } else if ids.len() == 7 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6]).query(conn.deref())?)
    } else if ids.len() == 8 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7]).query(conn.deref())?)
    } else if ids.len() == 9 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8]).query(conn.deref())?)
    } else if ids.len() == 10 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9]).query(conn.deref())?)
    } else if ids.len() == 11 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10]).query(conn.deref())?)
    } else if ids.len() == 12 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11]).query(conn.deref())?)
    } else if ids.len() == 13 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12]).query(conn.deref())?)
    } else if ids.len() == 14 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13]).query(conn.deref())?)
    } else if ids.len() == 15 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14]).query(conn.deref())?)
    } else if ids.len() == 16 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15]).query(conn.deref())?)
    } else if ids.len() == 17 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15], ids[16]).query(conn.deref())?)
    } else if ids.len() == 18 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15], ids[16],ids[17]).query(conn.deref())?)
    } else if ids.len() == 19 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15], ids[16],ids[17],ids[18]).query(conn.deref())?)
    } else if ids.len() == 20 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15], ids[16],ids[17],ids[18],ids[19]).query(conn.deref())?)
    } else if ids.len() == 21 {
        Ok(pipeline!(key; ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15], ids[16],ids[17],ids[18],ids[19],ids[20]).query(conn.deref())?)
    } else {
        Err(ServiceError::NoCacheFound)
    }
}

pub enum UpdateCache<'a> {
    GotTopic(&'a Topic),
    GotTopics(&'a Vec<Topic>),
    AddedTopic(&'a Topic, &'a Category),
    GotPost(&'a Post),
    GotPosts(&'a Vec<Post>),
    AddedPost(&'a Topic, &'a Category, &'a Post, &'a Option<Post>),
    GotUser(&'a User),
    GotCategories(&'a Vec<Category>),
    DeleteCategory(&'a u32),
}

type UpdateResult = Result<(), ServiceError>;

impl<'a> UpdateCache<'a> {
    pub fn handle_update(self, opt: &Option<&RedisPool>) -> UpdateResult {
        let conn = opt.unwrap().try_get().ok_or(ServiceError::CacheOffline)?;
        match self {
            UpdateCache::GotTopics(t) => update_hash_set(t, "topic", conn),
            UpdateCache::GotCategories(c) => update_hash_set(c, "category", conn),
            UpdateCache::GotPosts(p) => update_hash_set(&p, "post", conn),
            UpdateCache::AddedPost(t, c, p_new, p_old) => added_post(t, c, p_new, p_old, conn),
            UpdateCache::AddedTopic(t, c) => added_topic(t, c, conn),
            UpdateCache::GotPost(p) => Ok(conn.hset_multiple(&format!("post:{}:set", p.id), &p.sort_hash())?),
            UpdateCache::GotUser(u) => Ok(conn.hset_multiple(&format!("user:{}:set", u.id), &u.sort_hash())?),
            UpdateCache::GotTopic(t) => Ok(conn.hset_multiple(format!("topic:{}:set", t.id), &t.sort_hash())?),
            // ToDo: migrate post and topic when deleting category
            UpdateCache::DeleteCategory(id) => Ok(conn.del(format!("{}:{}:set", "category", id))?),
        }
    }
}

fn added_topic(topic: &Topic, category: &Category, conn: PoolConnectionRedis) -> UpdateResult {
    Ok(redis::pipe().atomic()
        .hset_multiple(format!("topic:{}:set", topic.id), &topic.sort_hash())
        .hset_multiple(format!("category:{}:set", category.id), &category.sort_hash())
        .lpush(format!("category:{}:list", category.id), topic.id)
        .query(conn.deref())?)
}

fn added_post(topic: &Topic, category: &Category, post: &Post, post_old: &Option<Post>, conn: PoolConnectionRedis) -> UpdateResult {
    let list_key = format!("category:{}:list", topic.category_id);
    Ok(match post_old {
        Some(p) => redis::pipe().atomic()
            .lrem(&list_key, 1, topic.id)
            .hset_multiple(format!("topic:{}:set", topic.id), &topic.sort_hash())
            .hset_multiple(format!("post:{}:set", post.id), &post.sort_hash())
            .hset_multiple(format!("post:{}:set", p.id), &p.sort_hash())
            .hset_multiple(format!("category:{}:set", category.id), &category.sort_hash())
            .rpush(format!("topic:{}:list", topic.id), post.id)
            .lpush(list_key, topic.id)
            .query(conn.deref())?,
        None => redis::pipe().atomic()
            .lrem(&list_key, 1, topic.id)
            .hset_multiple(format!("topic:{}:set", topic.id), &topic.sort_hash())
            .hset_multiple(format!("post:{}:set", post.id), &post.sort_hash())
            .hset_multiple(format!("category:{}:set", category.id), &category.sort_hash())
            .rpush(format!("topic:{}:list", topic.id), post.id)
            .lpush(list_key, topic.id)
            .query(conn.deref())?
    })
}

fn update_hash_set<T>(vec: &Vec<T>, key: &str, conn: PoolConnectionRedis) -> UpdateResult
    where T: GetSelfId + SortHash {
    macro_rules! pipeline {
        ( $ y: expr; $( $ x: expr),*) =>(redis::pipe().atomic() $ (.hset_multiple(&format!("{}:{}:set", $ y, $ x.get_self_id()), &$x.sort_hash()))*);
    }
    if vec.len() == 1 {
        Ok(pipeline![key; vec[0]].query(conn.deref())?)
    } else if vec.len() == 2 {
        Ok(pipeline![key; vec[0], vec[1]].query(conn.deref())?)
    } else if vec.len() == 3 {
        Ok(pipeline!(key; vec[0], vec[1], vec[2]).query(conn.deref())?)
    } else if vec.len() == 4 {
        Ok(pipeline!(key; vec[0], vec[1], vec[2], vec[3]).query(conn.deref())?)
    } else if vec.len() == 5 {
        Ok(pipeline!(key; vec[0], vec[1], vec[2], vec[3], vec[4]).query(conn.deref())?)
    } else if vec.len() == 6 {
        Ok(pipeline!(key; vec[0], vec[1], vec[2], vec[3], vec[4], vec[5]).query(conn.deref())?)
    } else if vec.len() == 7 {
        Ok(pipeline!(key; vec[0], vec[1], vec[2], vec[3], vec[4], vec[5], vec[6]).query(conn.deref())?)
    } else if vec.len() == 8 {
        Ok(pipeline!(key; vec[0], vec[1], vec[2], vec[3], vec[4], vec[5], vec[6], vec[7]).query(conn.deref())?)
    } else if vec.len() == 9 {
        Ok(pipeline!(key; vec[0], vec[1], vec[2], vec[3], vec[4], vec[5], vec[6], vec[7], vec[8]).query(conn.deref())?)
    } else if vec.len() == 10 {
        Ok(pipeline!(key; vec[0], vec[1], vec[2], vec[3], vec[4], vec[5], vec[6], vec[7], vec[8], vec[9]).query(conn.deref())?)
    } else if vec.len() == 11 {
        Ok(pipeline!(key; vec[0], vec[1], vec[2], vec[3], vec[4], vec[5], vec[6], vec[7], vec[8], vec[9], vec[10]).query(conn.deref())?)
    } else if vec.len() == 12 {
        Ok(pipeline!(key; vec[0], vec[1], vec[2], vec[3], vec[4], vec[5], vec[6], vec[7], vec[8], vec[9], vec[10], vec[11]).query(conn.deref())?)
    } else if vec.len() == 13 {
        Ok(pipeline!(key; vec[0], vec[1], vec[2], vec[3], vec[4], vec[5], vec[6], vec[7], vec[8], vec[9], vec[10], vec[11], vec[12]).query(conn.deref())?)
    } else if vec.len() == 14 {
        Ok(pipeline!(key; vec[0], vec[1], vec[2], vec[3], vec[4], vec[5], vec[6], vec[7], vec[8], vec[9], vec[10], vec[11], vec[12], vec[13]).query(conn.deref())?)
    } else if vec.len() == 15 {
        Ok(pipeline!(key; vec[0], vec[1], vec[2], vec[3], vec[4], vec[5], vec[6], vec[7], vec[8], vec[9], vec[10], vec[11], vec[12], vec[13], vec[14]).query(conn.deref())?)
    } else if vec.len() == 16 {
        Ok(pipeline!(key; vec[0], vec[1], vec[2], vec[3], vec[4], vec[5], vec[6], vec[7], vec[8], vec[9], vec[10], vec[11], vec[12], vec[13], vec[14], vec[15]).query(conn.deref())?)
    } else if vec.len() == 17 {
        Ok(pipeline!(key; vec[0], vec[1], vec[2], vec[3], vec[4], vec[5], vec[6], vec[7], vec[8], vec[9], vec[10], vec[11], vec[12], vec[13], vec[14], vec[15], vec[16]).query(conn.deref())?)
    } else if vec.len() == 18 {
        Ok(pipeline!(key; vec[0], vec[1], vec[2], vec[3], vec[4], vec[5], vec[6], vec[7], vec[8], vec[9], vec[10], vec[11], vec[12], vec[13], vec[14], vec[15], vec[16], vec[17]).query(conn.deref())?)
    } else if vec.len() == 19 {
        Ok(pipeline!(key; vec[0], vec[1], vec[2], vec[3], vec[4], vec[5], vec[6], vec[7], vec[8], vec[9], vec[10], vec[11], vec[12], vec[13], vec[14], vec[15], vec[16], vec[17], vec[18]).query(conn.deref())?)
    } else if vec.len() == 20 {
        Ok(pipeline!(key; vec[0], vec[1], vec[2], vec[3], vec[4], vec[5], vec[6], vec[7], vec[8], vec[9], vec[10], vec[11], vec[12], vec[13], vec[14], vec[15], vec[16], vec[17], vec[18], vec[19]).query(conn.deref())?)
    } else {
        Err(ServiceError::NoCacheFound)
    }
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
    Ok(redis::cmd("flushall").query(conn.deref())?)
}

// helper for mail service
pub enum MailCache<'a> {
    AddActivation(Mail<'a>),
    AddRecovery(Mail<'a>),
    GetActivation(Option<u32>),
    RemoveActivation(&'a u32),
    RemoveRecovery(&'a u32),
}

impl<'a> MailCache<'a> {
    pub fn modify(&self, pool: &Option<&RedisPool>) -> Result<(), ServiceError> {
        let pool = pool.unwrap();
        match self {
            MailCache::AddActivation(mail) => add_mail_cache(mail, "activation", pool.get()?),
            MailCache::RemoveActivation(id) => del_mail_queue(id, pool.get()?),
            _ => Ok(())
        }
    }
    pub fn get_mail_queue(&self, pool: &RedisPool) -> Result<String, ServiceError> {
        match self {
            MailCache::GetActivation(opt) => from_mail_queue(opt, pool.get()?),
            _ => Err(ServiceError::BadRequestGeneral)
        }
    }
    pub fn get_mail_hash(&self, pool: &RedisPool) -> Result<HashMap<String, String>, ServiceError> {
        match self {
            MailCache::GetActivation(opt) => from_mail_hash(opt, "activation", pool.get()?),
            _ => Err(ServiceError::BadRequestGeneral)
        }
    }
}

fn add_mail_cache(mail: &Mail, key: &str, conn: PoolConnectionRedis) -> Result<(), ServiceError> {
    let stringify = serde_json::to_string(mail)?;
    let key = format!("{}:{}:set", key, mail.user_id);
    Ok(redis::pipe().atomic()
        .zadd("mail_queue", stringify, mail.user_id)
        .hset_multiple(&key, &mail.sort_hash())
        .expire(&key, MAIL_LIFE)
        .query(conn.deref())?)
}

fn del_mail_queue(id: &u32, conn: PoolConnectionRedis) -> Result<(), ServiceError> {
    Ok(redis::cmd("zrembyscore").arg("mail_queue").arg(*id).arg(*id).query(conn.deref())?)
}

fn from_mail_queue(opt: &Option<u32>, conn: PoolConnectionRedis) -> Result<String, ServiceError> {
    match opt {
        Some(id) => Ok(redis::cmd("zrange").arg("mail_queue").arg(*id).arg(*id).query(conn.deref())?),
        None => Ok(redis::cmd("zrange").arg("mail_queue").arg(0).arg(1).query(conn.deref())?)
    }
}

fn from_mail_hash(opt: &Option<u32>, key: &str, conn: PoolConnectionRedis) -> Result<HashMap<String, String>, ServiceError> {
    let test = opt.unwrap();
    Ok(redis::cmd("hgetall").arg(format!("{}:{}:set", key, test)).query(conn.deref())?)
}