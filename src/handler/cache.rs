use std::collections::HashMap;
use futures::Future;

use actix_web::{web, HttpResponse, Error};
use chrono::NaiveDateTime;
use r2d2_redis::{redis, redis::{Commands, PipelineCommands}, RedisConnectionManager, redis::RedisError};
use serde::{Deserialize, Serialize};
use lazy_static::__Deref;

use crate::model::{
    errors::ServiceError,
    user::User,
    post::Post,
    topic::{Topic, TopicWithUser},
    category::Category,
    cache::{SortHash, FromHashMap},
    common::{RedisPool, PoolConnectionRedis, GetSelfId, AttachUser, get_unique_id},
};

const LIMIT: isize = 20;

pub enum GetCache {
    Topics,
    Topic,
    Users,
    Categories,
}

pub fn get_topic_cache(id: &u32, page: &i64, pool: &RedisPool) -> Result<HttpResponse, ServiceError> {
    let conn = pool.get()?;
    let topic = if page == &1 {
        get_topics(&vec![id.clone()], &conn)?.pop()
    } else { None };

    Ok(HttpResponse::Ok().finish())
}

pub fn handle_topics_cache(id: &u32, page: &i64, pool: &RedisPool) -> Result<HttpResponse, ServiceError> {
    let conn = pool.get()?;

    let list_key = format!("category:{}:list", id);
    let start = (*page as isize - 1) * 20;

    let topic_id: Vec<u32> = conn.lrange(&list_key, start, start + LIMIT - 1)?;
    let topics = get_topics(&topic_id, &conn)?;

    let user_id = get_unique_id(&topics, None);
    let users = get_users(&user_id, &conn)?;

    // ToDo: add trait for attach users hash to topic.
    Ok(HttpResponse::Ok().json(&topics.into_iter().map(|topic| topic.attach_user(&users)).collect::<Vec<TopicWithUser>>()))
}

fn get_posts(ids: &Vec<&u32>, conn: &PoolConnectionRedis) -> Result<Vec<User>, ServiceError> {
    let users_hash_vec = get_user_hash_set(&ids, &conn)?;

    if users_hash_vec.len() != ids.len() { return Err(ServiceError::NoCacheFound); };
    users_hash_vec.iter().map(|hash| {
        if hash.is_empty() { return Err(ServiceError::NoCacheFound); }
        hash.parse_user()
    }).collect::<Result<Vec<User>, ServiceError>>()
}

fn get_topics(ids: &Vec<u32>, conn: &PoolConnectionRedis) -> Result<Vec<Topic>, ServiceError> {
    let topics_hash_vec = get_topic_hash_set(&ids, &conn)?;
    if topics_hash_vec.len() != ids.len() { return Err(ServiceError::NoCacheFound); };

    topics_hash_vec.iter().map(|hash| {
        if hash.is_empty() { return Err(ServiceError::NoCacheFound); }
        hash.parse_topic()
    }).collect::<Result<Vec<Topic>, ServiceError>>()
}

fn get_users(ids: &Vec<&u32>, conn: &PoolConnectionRedis) -> Result<Vec<User>, ServiceError> {
    let users_hash_vec = get_user_hash_set(&ids, &conn)?;

    if users_hash_vec.len() != ids.len() { return Err(ServiceError::NoCacheFound); };
    users_hash_vec.iter().map(|hash| {
        if hash.is_empty() { return Err(ServiceError::NoCacheFound); }
        hash.parse_user()
    }).collect::<Result<Vec<User>, ServiceError>>()
}

fn get_user_hash_set(ids: &Vec<&u32>, conn: &PoolConnectionRedis) -> Result<Vec<HashMap<String, String>>, ServiceError> {
    macro_rules! pipe_topic {
        ($($x: expr),*) => {
        {
            redis::pipe().atomic()$(.hgetall(format!("user:{}:set", $x)))*
        }
        }}

    if ids.len() == 1 {
        Ok(pipe_topic![ids[0]].query(conn.deref())?)
    } else if ids.len() == 2 {
        Ok(pipe_topic![ids[0], ids[1]].query(conn.deref())?)
    } else if ids.len() == 3 {
        Ok(pipe_topic!(ids[0], ids[1], ids[2]).query(conn.deref())?)
    } else if ids.len() == 4 {
        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3]).query(conn.deref())?)
    } else if ids.len() == 5 {
        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4]).query(conn.deref())?)
    } else if ids.len() == 6 {
        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5]).query(conn.deref())?)
    } else if ids.len() == 7 {
        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6]).query(conn.deref())?)
    } else if ids.len() == 8 {
        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7]).query(conn.deref())?)
    } else if ids.len() == 9 {
        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8]).query(conn.deref())?)
    } else if ids.len() == 10 {
        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9]).query(conn.deref())?)
    } else if ids.len() == 11 {
        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10]).query(conn.deref())?)
    } else if ids.len() == 12 {
        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11]).query(conn.deref())?)
    } else if ids.len() == 13 {
        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12]).query(conn.deref())?)
    } else if ids.len() == 14 {
        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13]).query(conn.deref())?)
    } else if ids.len() == 15 {
        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14]).query(conn.deref())?)
    } else if ids.len() == 16 {
        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15]).query(conn.deref())?)
    } else if ids.len() == 17 {
        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15], ids[16]).query(conn.deref())?)
    } else if ids.len() == 18 {
        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15], ids[16],ids[17]).query(conn.deref())?)
    } else if ids.len() == 19 {
        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15], ids[16],ids[17],ids[18]).query(conn.deref())?)
    } else if ids.len() == 20 {
        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15], ids[16],ids[17],ids[18],ids[19]).query(conn.deref())?)
    } else {
        Err(ServiceError::NoCacheFound)
    }
}

fn get_topic_hash_set(ids: &Vec<u32>, conn: &PoolConnectionRedis) -> Result<Vec<HashMap<String, String>>, ServiceError> {
    ids.iter()
        .map(|id| Ok(redis::cmd("hgetall").arg(format!("topic:{}:set", id)).query(conn.deref())?))
        .collect::<Result<Vec<HashMap<String, String>>, ServiceError>>()
}


// ToDo: make a more compat macro to handle pipeline
//fn get_topic_hash_set(ids: &Vec<u32>, conn: &PoolConnectionRedis) -> Result<Vec<HashMap<String, String>>, ServiceError> {
//    macro_rules! pipe_topic {
//        ($($x: expr),*) => {
//        {
//            redis::pipe().atomic()$(.hgetall(format!("topic:{}:set", $x)))*
//        }
//        }}
//
//    if ids.len() == 1 {
//        Ok(pipe_topic![ids[0]].query(conn.deref())?)
//    } else if ids.len() == 2 {
//        Ok(pipe_topic![ids[0], ids[1]].query(conn.deref())?)
//    } else if ids.len() == 3 {
//        Ok(pipe_topic!(ids[0], ids[1], ids[2]).query(conn.deref())?)
//    } else if ids.len() == 4 {
//        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3]).query(conn.deref())?)
//    } else if ids.len() == 5 {
//        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4]).query(conn.deref())?)
//    } else if ids.len() == 6 {
//        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5]).query(conn.deref())?)
//    } else if ids.len() == 7 {
//        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6]).query(conn.deref())?)
//    } else if ids.len() == 8 {
//        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7]).query(conn.deref())?)
//    } else if ids.len() == 9 {
//        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8]).query(conn.deref())?)
//    } else if ids.len() == 10 {
//        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9]).query(conn.deref())?)
//    } else if ids.len() == 11 {
//        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10]).query(conn.deref())?)
//    } else if ids.len() == 12 {
//        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11]).query(conn.deref())?)
//    } else if ids.len() == 13 {
//        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12]).query(conn.deref())?)
//    } else if ids.len() == 14 {
//        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13]).query(conn.deref())?)
//    } else if ids.len() == 15 {
//        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14]).query(conn.deref())?)
//    } else if ids.len() == 16 {
//        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15]).query(conn.deref())?)
//    } else if ids.len() == 17 {
//        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15], ids[16]).query(conn.deref())?)
//    } else if ids.len() == 18 {
//        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15], ids[16],ids[17]).query(conn.deref())?)
//    } else if ids.len() == 19 {
//        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15], ids[16],ids[17],ids[18]).query(conn.deref())?)
//    } else if ids.len() == 20 {
//        Ok(pipe_topic!(ids[0], ids[1], ids[2], ids[3], ids[4],ids[5], ids[6], ids[7], ids[8], ids[9],ids[10],ids[11], ids[12], ids[13],ids[14],ids[15], ids[16],ids[17],ids[18],ids[19]).query(conn.deref())?)
//    } else {
//        Err(ServiceError::NoCacheFound)
//    }
//}


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
            UpdateCache::Topics(topics) => update_cache(&topics, "topic", &conn),
            UpdateCache::Posts(posts) => update_cache(&posts, "post", &conn),
            UpdateCache::Users(users) => update_cache(&users, "user", &conn),
            UpdateCache::Categories(categories) => update_cache(&categories, "category", &conn),
            UpdateCache::DeleteCategory(id) => delete_hash_set(&id, "category", &conn)
        }
    }
}

pub fn update_cache<T>(vec: &Vec<T>, key: &str, conn: &PoolConnectionRedis) -> UpdateResult
    where T: SortHash + GetSelfId {
    let _result = vec.iter().map(|v| {
        let set_key = format!("{}:{}:set", key, v.get_self_id());
        let hash_set = v.sort_hash();
        conn.hset_multiple(&set_key, &hash_set)?;
        Ok(())
    }).collect::<Result<(), ServiceError>>()?;
    Ok(())
}

fn delete_hash_set(id: &u32, key: &str, conn: &PoolConnectionRedis) -> UpdateResult {
    let key = format!("{}:{}:set", key, id);
    conn.del(key)?;
    Ok(())
}

pub fn build_list(ids: Vec<u32>, foreign_key: &str, conn: &PoolConnectionRedis) -> UpdateResult {
    let key = format!("{}:list", foreign_key);
    ids.into_iter().map(|id| {
        conn.rpush(&key, id)?;
        Ok(())
    }).collect::<Result<(), ServiceError>>()
}

pub fn clear_cache(pool: &RedisPool) -> Result<(), ServiceError> {
    let conn = pool.get()?;
    redis::cmd("flushall").query(&*conn)?;
    Ok(())
}