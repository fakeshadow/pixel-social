use actix_web::{web, HttpResponse};
use r2d2_redis::{redis, RedisConnectionManager};
use r2d2_redis::redis::{Commands, RedisError};
use serde::{Deserialize, Serialize};
use serde_json as json;
use lazy_static::__Deref;

use crate::model::{
    errors::ServiceError,
    user::PublicUser,
    post::Post,
    topic::{TopicWithPost, TopicWithUser, Topic},
    cache::{CacheQuery, CacheQueryResult, TopicCacheRequest, CategoryCacheRequest},
    common::{RedisPool, GetSelfId, get_unique_id},
};
use crate::model::common::{AttachPublicUserRef, GetSelfUser, GetSelfTopicPost};

const LIMIT: isize = 20;

//type QueryResult = Result<HttpResponse, ServiceError>;
type QueryResult = Result<(), ServiceError>;
type Conn = redis::Connection;

//impl<'a> CacheQuery<'a> {
//    pub fn handle_query(self, cache_pool: &web::Data<RedisPool>) -> QueryResult {
//        let conn = &cache_pool.try_get().ok_or(ServiceError::RedisOffline)?;
//        match self {
//            CacheQuery::GetTopic(cache_request) => get_topic_cache(&cache_request, &conn),
//            CacheQuery::GetCategory(cache_request) => get_category_cache(&cache_request, &conn),
//            CacheQuery::UpdateCategory(topics) => update_category_cache(&topics, &conn),
//            CacheQuery::UpdateTopic(topic_with_post) => update_topic_cache(&topic_with_post, &conn),
//        }
//    }
//}

//pub fn cache_handler(query: &CacheQuery, cache_pool: &) -> impl


//fn get_topic_cache(cache_request: &TopicCacheRequest, conn: &Conn) -> QueryResult {
//    // get topic from redis
//    let topic_id = cache_request.topic.clone();
//    let page = cache_request.page;
//
//    // get Option<Topic> if the page is 1. return None if it's not
//    let topic = if page == &1isize {
//        let category_key = format!("category:{}", 1);
//        // ToDo: in case vec len is 0 and cause panic
//        let topics_string = from_score(&category_key, topic_id, topic_id, &conn)?;
//        let mut topic_vec: Vec<Topic> = deserialize_string_vec(&topics_string)?;
//        let topic_user_vec = get_users(&topic_vec, &conn)?;
//        let topic = topic_vec.pop().ok_or(ServiceError::NoCacheFound)?;
//        Some(topic.attach_from_public(&topic_user_vec))
//    } else { None };
//
//    let offset = (page - 1) * 20;
//    let topic_key = format!("topic:{}", &topic_id);
//
//    let posts_string = from_range(&topic_key, "zrange", offset, &conn)?;
//    let post_vec: Vec<Post> = deserialize_string_vec(&posts_string)?;
//    let post_user_vec = get_users(&post_vec, &conn)?;
//
//    let posts = Some(post_vec.into_iter().map(|post| post.to_ref().attach_user(&post_user_vec)).collect());
//
////    Ok(CacheQueryResult::GotTopic(TopicWithPost::new(topic, posts)).to_response())
//    Ok(())
//}
//
//fn get_category_cache(cache_request: &CategoryCacheRequest, conn: &Conn) -> QueryResult {
//    let page = cache_request.page;
//    let categories = cache_request.categories;
//    let offset = (page - 1) * 20;
//    // ToDo: For now only query the first category from request.
//    let category_key = format!("category:{}", categories[0]);
//    let topics_string = from_range(&category_key, "zrevrange", offset, &conn)?;
//    if topics_string.len() == 0 { return Err(ServiceError::NotFound); }
//
//    let topics_vec: Vec<Topic> = deserialize_string_vec(&topics_string)?;
//    let users = get_users(&topics_vec, &conn)?;
//
////    let topics_with_user: Vec<TopicWithUser> = topics_vec.into_iter().map(|topic| topic.attach_from_public(&users)).collect();
//
//    let mut topics: Vec<TopicWithUser> = Vec::with_capacity(20);
//    for topic in topics_vec.iter() {
//        topics.push(topic.to_ref().attach_user(&users))
//    }
//
////    Ok(CacheQueryResult::GotCategory(&topics).to_response())
//    Ok(())
//}

//fn update_category_cache(topics: &Vec<TopicWithUser>, conn: &Conn) -> QueryResult {
//    let category_id = topics[0].topic.category_id;
//    let category_key = format!("category:{}", &category_id);
//    let (topic_rank_vec, user_rank_vec) = serialize_vec(&topics, None)?;
////    conn.zadd_multiple("users", &user_rank_vec)?;
////    conn.zadd_multiple(category_key, &topic_rank_vec)?;
////    Ok(CacheQueryResult::Updated.to_response())
//    Ok(())
//}

//fn update_topic_cache(topic_with_post: &TopicWithPost, conn: &Conn) -> QueryResult {
//    let mut topic_rank_vec: Vec<(u32, String)> = Vec::with_capacity(1);
//    let mut post_rank_vec: Vec<(u32, String)> = Vec::with_capacity(20);
//    let mut user_rank_vec: Vec<(u32, String)> = Vec::with_capacity(21);
//
//    if let Some(topic) = &topic_with_post.topic {
//        topic_rank_vec.push((
//            topic.get_self_id_copy(),
//            json::to_string(&topic.topic)?,
//        ));
//        if let Some(user_id) = topic.get_self_user_id() {
//            user_rank_vec.push((user_id, json::to_string(&topic.user)?));
//        }
//    }
//    if let Some(posts_with_user) = &topic_with_post.posts {
//        // ToDo: In case panic
//        let topic_user = user_rank_vec.pop();
//        let (post_rank, user_rank) =
//            serialize_vec(&posts_with_user, topic_user)?;
//        post_rank_vec = post_rank;
//        user_rank_vec = user_rank;
//    }
//    if !topic_rank_vec.is_empty() {
//        let category_key =
//            format!("category:{}", topic_with_post.get_category_id().ok_or(ServiceError::NoCacheFound)?);
//        conn.zadd_multiple(category_key, &topic_rank_vec)?;
//    }
//    if !user_rank_vec.is_empty() {
//        conn.zadd_multiple("users", &user_rank_vec)?;
//    }
//    if !post_rank_vec.is_empty() {
//        let topic_key =
//            format!("topic:{}", topic_with_post.get_topic_id().ok_or(ServiceError::NoCacheFound)?);
//        conn.zadd_multiple(topic_key, &post_rank_vec)?;
//    }
////    Ok(CacheQueryResult::Updated.to_response())
//    Ok(())
//}

/// pass topic user id/string tuple as an option.
//fn serialize_vec<T, R, E>(
//    vec: &Vec<T>,
//    topic_user: Option<(u32, String)>,
//) -> Result<(Vec<(u32, String)>, Vec<(u32, String)>), ServiceError>
//    where T: GetSelfId + GetSelfUser<R> + GetSelfTopicPost<E> {
//    let mut topics_or_posts_rank: Vec<(u32, String)> = Vec::with_capacity(20);
//    let mut users_rank: Vec<(u32, String)> = Vec::with_capacity(21);
//
//    if let Some(tuple) = topic_user { users_rank.push(tuple) };
//
//    for item in vec.iter() {
//
//
//        if let Some(user_id) = item.get_self_user_id() {
//            let tuple = (user_id, json::to_string(&item.get_self_user().ok_or(ServiceError::NoCacheFound)?)?);
//            if !users_rank.contains(&tuple) {
//                users_rank.push(tuple)
//            }
//        }
//        topics_or_posts_rank.push((
//            *item.get_self_id(),
//            json::to_string(&item.get_self_topic_post())?,
//        ));
//    }
//    Ok((topics_or_posts_rank, users_rank))
//}

//fn get_users<'a, T, R>(vec: &Vec<T>, conn: &redis::Connection) -> Result<Vec<PublicUser>, ServiceError>
//    where T: AttachPublicUserRef<R> {
//    let mut users_id = get_unique_id(&vec, None);
//    if users_id.is_empty() { return Ok(vec![]); }
//    users_id.sort();
//
//    let range_index = users_id.len() - 1;
//    let range_start = users_id[0].clone();
//    let range_end = users_id[range_index].clone();
//
//    let users_vec: Vec<(String, u32)> = conn.zrangebyscore_withscores("users", range_start, range_end)?;
//
//    let mut users: Vec<PublicUser> = Vec::with_capacity(20);
//    for _user in users_vec.iter() {
//        let (_user_string, _user_id) = _user;
//        if users_id.contains(&_user_id) {
//            users.push(json::from_str(_user_string)?)
//        }
//    };
//    Ok(users)
//}

fn from_score(key: &str, start_score: u32, end_score: u32, conn: &Conn) -> Result<Vec<String>, ServiceError> {
    let vec = redis::cmd("zrangebyscore")
        .arg(key)
        .arg(start_score)
        .arg(end_score)
        .query(conn.deref())?;
    Ok(vec)
}

fn from_range(key: &str, cmd: &str, offset: isize, conn: &Conn) -> Result<Vec<String>, ServiceError> {
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