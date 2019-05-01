use std::collections::HashMap;
use futures::Future;

use actix_web::{web, HttpResponse, Error};
use chrono::NaiveDateTime;
use r2d2_redis::{redis, redis::{Commands, PipelineCommands}, RedisConnectionManager, redis::RedisError};
use serde::{Deserialize, Serialize};
use serde_json as json;
use lazy_static::__Deref;

use crate::model::{
    errors::ServiceError,
    user::{User, PublicUser},
    post::Post,
    topic::Topic,
    category::Category,
    cache::SortHash,
    common::{RedisPool, PoolConnectionRedis, GetSelfId, AttachUser, get_unique_id},
};
use std::thread;
use crate::model::cache::FromHashMap;

const LIMIT: isize = 20;

type QueryResult = Result<(), ServiceError>;


pub fn get_topics_cache(id: &u32, page: &i64, pool: &RedisPool) -> Result<Vec<Topic>, ServiceError> {
    let conn = pool.get()?;

    let list_key = format!("category:{}:list", id);
    let start = (*page as isize - 1) * 20;

    let ids: Vec<u32> = conn.lrange(&list_key, start, start + LIMIT - 1)?;
    let hash_vec = get_hash_set(&ids, conn)?;
    if hash_vec.len() != ids.len() { return Err(ServiceError::NoCacheFound); };

    hash_vec.iter().map(|hash| {
        if hash.is_empty() { return Err(ServiceError::NoCacheFound); }
        hash.map_topic()
    }).collect::<Result<Vec<Topic>, ServiceError>>()
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
    Ok(())
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

// ToDo: make a more compat macro to handle pipeline
fn get_hash_set(ids: &Vec<u32>, conn: PoolConnectionRedis) -> Result<Vec<HashMap<String, String>>, ServiceError> {
    macro_rules! pipeline {
        ($($x: expr),*) => {
        {
            redis::pipe()$(.hgetall(format!("topic:{}:set", $x)))*
        }}
    }
    if ids.len() == 1 {
        Ok(pipeline!(ids[0]).query(&*conn)?)
    } else if ids.len() == 2 {
        Ok(pipeline!(ids[0], ids[1]).query(&*conn)?)
    } else if ids.len() == 3 {
        Ok(pipeline!(ids[0], ids[1], ids[2]).query(&*conn)?)
    } else if ids.len() == 4 {
        Ok(pipeline!(ids[0], ids[1], ids[2], ids[3]).query(&*conn)?)
    } else if ids.len() == 5 {
        Ok(pipeline!(ids[0], ids[1], ids[2], ids[3], ids[4]).query(&*conn)?)
    } else if ids.len() == 6 {
        Ok(pipeline!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5]).query(&*conn)?)
    } else if ids.len() == 7 {
        Ok(pipeline!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6]).query(&*conn)?)
    } else if ids.len() == 8 {
        Ok(pipeline!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7]).query(&*conn)?)
    } else if ids.len() == 9 {
        Ok(pipeline!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8]).query(&*conn)?)
    } else if ids.len() == 10 {
        Ok(pipeline!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9]).query(&*conn)?)
    } else if ids.len() == 11 {
        Ok(pipeline!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10]).query(&*conn)?)
    } else if ids.len() == 12 {
        Ok(pipeline!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11]).query(&*conn)?)
    } else if ids.len() == 13 {
        Ok(pipeline!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12]).query(&*conn)?)
    } else if ids.len() == 14 {
        Ok(pipeline!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13]).query(&*conn)?)
    } else if ids.len() == 15 {
        Ok(pipeline!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14]).query(&*conn)?)
    } else if ids.len() == 16 {
        Ok(pipeline!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15]).query(&*conn)?)
    } else if ids.len() == 17 {
        Ok(pipeline!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15], ids[16]).query(&*conn)?)
    } else if ids.len() == 18 {
        Ok(pipeline!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15], ids[16],ids[17]).query(&*conn)?)
    } else if ids.len() == 19 {
        Ok(pipeline!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15], ids[16],ids[17],ids[18]).query(&*conn)?)
    } else if ids.len() == 20 {
        Ok(pipeline!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15], ids[16],ids[17],ids[18],ids[19]).query(&*conn)?)
    } else {
        Err(ServiceError::NoCacheFound)
    }
}