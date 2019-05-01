use std::collections::HashMap;
use futures::Future;

use actix_web::{web, HttpResponse, Error};
use chrono::NaiveDateTime;
use r2d2_redis::{redis, redis::Commands, RedisConnectionManager, redis::RedisError};
use serde::{Deserialize, Serialize};
use serde_json as json;
use lazy_static::__Deref;

use crate::model::{
    errors::ServiceError,
    user::{User,PublicUser},
    post::Post,
    topic::Topic,
    category::Category,
    cache::SortHash,
    common::{RedisPool, PoolConnectionRedis, GetSelfId, AttachUser, get_unique_id},
};

const LIMIT: isize = 20;

type QueryResult = Result<(), ServiceError>;


pub fn get_topics_cache(id: &u32, page: &i64, pool: &RedisPool) -> Result<Vec<Topic>, ServiceError> {
    let conn = pool.get()?;

    let key = format!("topic:{}:set", 18324);
    let hash: HashMap<String, String> = conn.hgetall(&key)?;
    if hash.is_empty() { return Err(ServiceError::NoCacheFound); }

    let template_date = NaiveDateTime::from_timestamp(0, 0);
    let topic = topic_from_hash_map(hash, template_date.clone())?;

    Ok(vec![topic])
}

fn topic_from_hash_map(hash: HashMap<String, String>, date: NaiveDateTime) -> Result<Topic, ServiceError> {
    let mut topic = Topic {
        id: 0,
        user_id: 0,
        category_id: 0,
        title: "".to_string(),
        body: "".to_string(),
        thumbnail: "".to_string(),
        created_at: date,
        updated_at: date,
        last_reply_time: date,
        reply_count: 0,
        is_locked: false,
    };

    for (r, v) in hash.into_iter() {
        match r.as_str() {
            "id" => topic.id = v.parse::<u32>().ok().ok_or(ServiceError::InternalServerError)?,
            "user_id" => topic.user_id = v.parse::<u32>().ok().ok_or(ServiceError::InternalServerError)?,
            "category_id" => topic.category_id = v.parse::<u32>().unwrap(),
            "title" => topic.title = v,
            "body" => topic.body = v,
            "thumbnail" => topic.thumbnail = v,
            "created_at" => topic.created_at = NaiveDateTime::parse_from_str(&v, "%Y-%m-%d %H:%M:%S%.f").ok().ok_or(ServiceError::InternalServerError)?,
            "updated_at" => topic.updated_at = NaiveDateTime::parse_from_str(&v, "%Y-%m-%d %H:%M:%S%.f").ok().ok_or(ServiceError::InternalServerError)?,
            "last_reply_time" => topic.last_reply_time = NaiveDateTime::parse_from_str(&v, "%Y-%m-%d %H:%M:%S%.f").ok().ok_or(ServiceError::InternalServerError)?,
            "reply_count" => topic.reply_count = v.parse::<i32>().ok().ok_or(ServiceError::InternalServerError)?,
            "is_locked" => topic.is_locked = v.parse::<bool>().ok().ok_or(ServiceError::InternalServerError)?,
            _ => return Err(ServiceError::NoCacheFound)
        }
    }
    Ok(topic)
}

pub enum UpdateCache<'a> {
    Topics(&'a Vec<Topic>),
    Posts(&'a Vec<Post>),
    Users(&'a Vec<User>),
    Categories(&'a Vec<Category>),
    DeleteCategory(&'a u32),
}

type UpdateResult = Result<(), ServiceError>;

impl<'a> UpdateCache<'a> {
    pub fn handle_update(&self, opt: &Option<&RedisPool>) -> UpdateResult {
        let conn = opt.unwrap().get()?;
        match self {
            UpdateCache::Topics(topics) => update_topics(&topics, &conn),
            UpdateCache::Posts(posts) => update_posts(&posts, &conn),
            UpdateCache::Users(users) => update_users(&users, &conn),
            UpdateCache::Categories(categories) => update_categories_cache(&categories, &conn),
            UpdateCache::DeleteCategory(id) => delete_hash_set(&id, "category", &conn)
        }
    }
}

pub fn update_categories_cache(categories: &Vec<Category>, conn: &PoolConnectionRedis) -> UpdateResult {
    let _result = categories.iter().map(|category| {
        let set_key = format!("category:{}:set", category.get_self_id());
        let hash_set = category.sort_hash();
        conn.hset_multiple(&set_key, &hash_set)?;
        Ok(())
    }).collect::<Result<(), ServiceError>>()?;
    Ok(())
}

pub fn build_list(ids: Vec<u32>, foreign_key: &str, conn: &PoolConnectionRedis) -> UpdateResult {
    let key = format!("{}:list", foreign_key);
    ids.into_iter().map(|id| {
        conn.rpush(&key, id)?;
        Ok(())
    }).collect::<Result<(), ServiceError>>()
}

fn delete_hash_set(id: &u32, key: &str, conn: &PoolConnectionRedis) -> UpdateResult {
    let key = format!("{}:{}:set", key, id);
    conn.del(key)?;
    Ok(())
}

fn get_users(min_id: u32, max_id: u32, conn: &PoolConnectionRedis) -> Result<Vec<PublicUser>, ServiceError> {
    let vec: Vec<String> = conn.zrangebyscore("users", min_id, max_id)?;
    println!("{:?}",vec);
    Ok(deserialize_string_vec::<PublicUser>(&vec)?)
}

pub fn update_users(users: &Vec<User>, conn: &PoolConnectionRedis) -> UpdateResult {
    let rank = users.iter().map(|user| user.sort_hash())
        .collect::<Result<Vec<(u32, String)>, ServiceError>>()?;
    conn.zadd_multiple("users", &rank)?;
    Ok(())
}

fn update_posts(posts: &Vec<Post>, conn: &PoolConnectionRedis) -> UpdateResult {
    let _result = posts.iter().map(|post| {
        let set_key = format!("post:{}:set", post.get_self_id());
        let hash_set = post.sort_hash();
        conn.hset_multiple(&set_key, &hash_set)?;
        Ok(())
    }).collect::<Result<(), ServiceError>>()?;
    Ok(())
}

fn update_topics(topics: &Vec<Topic>, conn: &PoolConnectionRedis) -> UpdateResult {
    let _result = topics.iter().map(|topic| {
        let set_key = format!("topic:{}:set", topic.get_self_id());
        let hash_set = topic.sort_hash();
        conn.hset_multiple(&set_key, &hash_set)?;
        Ok(())
    }).collect::<Result<(), ServiceError>>()?;

    let key = format!("topic:18324:set");
    let test: HashMap<String, String> = conn.hgetall(&key)?;

    Ok(())
}

//helper functions
fn from_score(key: &str, start_score: u32, end_score: u32, conn: &PoolConnectionRedis) -> UpdateResult {
    let vec = redis::cmd("zrangebyscore")
        .arg(key)
        .arg(start_score)
        .arg(end_score)
        .query(conn.deref())?;
    Ok(vec)
}

fn from_range(key: &str, cmd: &str, offset: isize, conn: &PoolConnectionRedis) -> UpdateResult {
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