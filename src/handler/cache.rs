use actix_web::{web, HttpResponse};
use r2d2_redis::{redis, redis::Commands, RedisConnectionManager};
use serde::{Deserialize, Serialize};
use serde_json as json;
use lazy_static::__Deref;

use crate::model::{
    errors::ServiceError,
    user::User,
    post::Post,
    topic::Topic,
    category::Category,
    cache::SortHash,
    common::{RedisPool, PoolConnectionRedis, GetSelfId, AttachUserRef, get_unique_id},
};

const LIMIT: isize = 20;

type QueryResult = Result<(), ServiceError>;


pub enum UpdateCache<'a> {
    Topics(&'a Vec<Topic>),
    Posts(&'a Vec<Post>),
    Users(&'a Vec<User>),
    Categories(&'a Vec<Category>),
}

type UpdateResult = Result<(), ServiceError>;

impl<'a> UpdateCache<'a> {
    pub fn handle_update(&self, opt: &Option<&RedisPool>) -> UpdateResult {
        let conn = opt.unwrap().get()?;
        match self {
            UpdateCache::Topics(topics) => update_topics(&topics, "topics", &conn),
            UpdateCache::Posts(posts) => update_posts(&posts, "posts", &conn),
            UpdateCache::Users(users) => update_users(&users, &conn),
            _ => Ok(())
        }
    }
}

fn update_users(users: &Vec<User>, conn: &PoolConnectionRedis) -> Result<(), ServiceError> {
    let rank = users.iter().map(|user| user.sort_hash())
        .collect::<Result<Vec<(u32, String)>, ServiceError>>()?;
    // ToDo: check existing score and update existing score;
    conn.zadd_multiple("users", &rank)?;
    Ok(())
}

fn update_posts(posts: &Vec<Post>, key: &str, conn: &PoolConnectionRedis) -> Result<(), ServiceError> {
    let _result = posts.iter().map(|post| {
        let set_key = format!("{}:{}:set", key, post.get_self_id());
        let hash_set = post.sort_hash()?;
        conn.hset_multiple(&set_key, &hash_set)?;
        Ok(())
    }).collect::<Result<(), ServiceError>>()?;
    Ok(())
}

fn update_topics(topics: &Vec<Topic>, key: &str, conn: &PoolConnectionRedis) -> Result<(), ServiceError> {
    let _result = topics.iter().map(|topic| {
        let set_key = format!("{}:{}:set", key, topic.get_self_id());
        let hash_set = topic.sort_hash()?;
        conn.hset_multiple(&set_key, &hash_set)?;
        Ok(())
    }).collect::<Result<(), ServiceError>>()?;
    Ok(())
}

//fn update_categories(categories: &Vec<Category>, key: &str, conn: &PoolConnectionRedis) -> Result<(), ServiceError> {
//    let _result = categories.iter().map(|category| {
//        let set_key = format!("{}:{}:set", key, category.get_self_id());
//        let hash_set = category.sort_hash()?;
//        conn.hset_multiple(&set_key, &hash_set)?;
//        Ok(())
//    }).collect::<Result<(), ServiceError>>()?;
//
//    Ok(())
//}


//helper functions
fn from_score(key: &str, start_score: u32, end_score: u32, conn: &PoolConnectionRedis) -> Result<Vec<String>, ServiceError> {
    let vec = redis::cmd("zrangebyscore")
        .arg(key)
        .arg(start_score)
        .arg(end_score)
        .query(conn.deref())?;
    Ok(vec)
}

fn from_range(key: &str, cmd: &str, offset: isize, conn: &PoolConnectionRedis) -> Result<Vec<String>, ServiceError> {
    let vec = redis::cmd(cmd)
        .arg(key)
        .arg(offset)
        .arg(offset + LIMIT)
        .query(conn.deref())?;
    Ok(vec)
}

fn deserialize_string_vec<'a, T>(vec: &'a Vec<String>) -> Result<Vec<T>, serde_json::Error>
    where T: Deserialize<'a> {
    vec.iter().map(|string| json::from_str(&string))
        .collect::<Result<Vec<T>, serde_json::Error>>()
}

pub fn clear_cache(pool: &RedisPool) -> Result<(), ServiceError> {
    let conn = pool.get()?;
    redis::cmd("flushall").query(&*conn)?;
    Ok(())
}