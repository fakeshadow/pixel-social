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
    cache::{TopicHashSet, TopicRankSet, SortHash},
    common::{RedisPool, ToHashSet, ToRankSet, PoolConnectionRedis, GetSelfId, AttachUserRef, get_unique_id},
};

const LIMIT: isize = 20;

type QueryResult = Result<(), ServiceError>;


pub enum UpdateCache<'a> {
    Topics(&'a Vec<Topic>),
    Posts(&'a Vec<Post>),
    Users(&'a Vec<User>),
    Categories(&'a Vec<Category>,),
}

type UpdateResult = Result<(), ServiceError>;

impl<'a> UpdateCache<'a> {
    pub fn handle_update(&self, opt: &Option<&RedisPool>) -> UpdateResult{
        let conn = opt.unwrap().get()?;
        match self {
            UpdateCache::Topics(topics) => update_topics(&topics, &conn),
            UpdateCache::Posts(posts) => update_posts(&posts, &conn),
            UpdateCache::Users(users) => update_users(&users, &conn),
            _=> Ok(())
        }
    }
}

fn update_users(topics: &Vec<User>, conn: &PoolConnectionRedis) -> Result<(), ServiceError> {

    let (_, rank_set) = to_sets(&topics);
    let rank = serialize_vec(&rank_set)?;

    // ToDo: check existing score and update existing score;
    conn.zadd_multiple("posts", &rank)?;

    Ok(())
}


fn update_posts(topics: &Vec<Post>, conn: &PoolConnectionRedis) -> Result<(), ServiceError> {

//    let (hash_set, rank_set) = to_sets(&topics);
//    let rank = serialize_vec(&rank_set)?;
//
//    for hash in hash_set.iter() {
//        let key = format!("post:{}:set", hash.get_self_id());
//        let sorted = hash.sort_hash()?;
//        conn.hset_multiple(&key, &sorted)?;
//    }
//    // ToDo: check existing score and update existing score;
//    conn.zadd_multiple("posts", &rank)?;

    Ok(())
}


fn update_topics(topics: &Vec<Topic>, conn: &PoolConnectionRedis) -> Result<(), ServiceError> {

    let (hash_set, rank_set) = to_sets(&topics);
    let rank = serialize_vec(&rank_set)?;

    for hash in hash_set.iter() {
        let key = format!("topic:{}:set", hash.get_self_id());
        let sorted = hash.sort_hash()?;
        conn.hset_multiple(&key, &sorted)?;
    }
    // ToDo: check existing score and update existing score;
    conn.zadd_multiple("topics", &rank)?;

    Ok(())
}

// helper functions
fn to_sets<'a, T>(vec: &'a Vec<T>) -> (Vec<<T as ToHashSet>::Output>, Vec<<T as ToRankSet<'a>>::Output>)
    where T: ToHashSet<'a> + ToRankSet<'a> {
    let mut hash_sets = Vec::with_capacity(20);
    let mut rank_set = Vec::with_capacity(20);
    for item in vec.iter() {
        hash_sets.push(item.to_hash());
        rank_set.push(item.to_rank());
    }
    (hash_sets, rank_set)
}

fn serialize_vec<T>(items: &Vec<T>) -> Result<Vec<(u32, String)>, ServiceError>
    where T: GetSelfId + Serialize {
    let mut result: Vec<(u32, String)> = Vec::new();
    for item in items.iter() {
        result.push((item.get_self_id().clone(), json::to_string(item)?));
    }
    Ok(result)
}


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
    vec.iter().map(|topic_string| json::from_str(&topic_string))
        .collect::<Result<Vec<T>, serde_json::Error>>()
}