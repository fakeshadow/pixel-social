use actix_web::{web, HttpResponse};
use r2d2_redis::redis;
use r2d2_redis::redis::{Commands, RedisError};
use serde_json as json;

use crate::model::{
    cache::{CacheQuery, CacheQueryResult},
    common::*,
    errors::ServiceError,
    post::*,
    topic::*,
    user::*,
};

use lazy_static::__Deref;

const LIMIT: isize = 20;

pub fn cache_handler(
    query: CacheQuery,
    pool: &web::Data<RedisPool>,
) -> Result<CacheQueryResult, ServiceError> {
    match &pool.try_get() {
        None => Err(ServiceError::RedisOffline),
        Some(conn) => {
            match query {
                CacheQuery::GetTopic(cache_request) => {
                    // get topic from redis
                    let topic_id = cache_request.topic;
                    let page = cache_request.page;

                    // get Option<Topic> if the page is 1. return None if it's not
                    let topic = if page == &1isize {
                        let category_key = format!("category:{}", 1);
                        let topics_string_vec: Vec<String> = redis::cmd("zrangebyscore")
                            .arg(&category_key)
                            .arg(topic_id.clone())
                            .arg(topic_id.clone())
                            .query(conn.deref())?;

                        if topics_string_vec.is_empty() { return Err(ServiceError::NotFound); };

                        let mut topic_vec: Vec<Topic> =
                            topics_string_vec
                                .into_iter()
                                .map(|topic_string| json::from_str(&topic_string))
                                .collect::<Result<Vec<Topic>, serde_json::Error>>()?;

                        let topic_user_vec = get_users(&topic_vec, &conn)?;

                        let topic = match topic_vec.pop() {
                            Some(topic) => topic,
                            None => return Err(ServiceError::NotFound)
                        };
                        Some(topic.attach_user(&topic_user_vec))
                    } else { None };

                    //get posts from redis. need to improve the code to be parallel with topic query
                    let offset = (page - 1) * 20;
                    let topic_key = format!("topic:{}", &topic_id);

                    let post_string_vec: Vec<String> = redis::cmd("zrange")
                        .arg(&topic_key)
                        .arg(offset)
                        .arg(offset + LIMIT)
                        .query(conn.deref())?;

                    let post_vec: Vec<Post> = post_string_vec
                        .into_iter()
                        .map(|post_string| json::from_str(&post_string))
                        .collect::<Result<Vec<Post>, serde_json::Error>>()?;

                    let post_user_vec = get_users(&post_vec, &conn)?;

                    let posts = Some(post_vec.into_iter().map(|post| post.attach_user(&post_user_vec)).collect());
                    Ok(CacheQueryResult::GotTopic(
                        TopicWithPost {
                            topic,
                            posts,
                        }))
                }

                CacheQuery::GetCategory(cache_request) => {
                    let page = cache_request.page;
                    let categories = cache_request.categories;

                    let offset = (page - 1) * 20;
                    let category_key = format!("category:{}", categories[0]);
                    let topics_string_vec: Vec<String> = redis::cmd("zrevrange")
                        .arg(category_key)
                        .arg(offset)
                        .arg(offset + LIMIT)
                        .query(conn.deref())?;

                    if topics_string_vec.len() == 0 { return Err(ServiceError::NotFound); }

                    let topics_vec: Vec<Topic> =
                        topics_string_vec
                            .into_iter()
                            .map(|topic_string| json::from_str(&topic_string))
                            .collect::<Result<Vec<Topic>, serde_json::Error>>()?;

                    let users = get_users(&topics_vec, &conn)?;

                    let topics_with_user: Vec<TopicWithUser<SlimUser>> =
                        topics_vec
                            .into_iter()
                            .map(|topic| topic.attach_user(&users))
                            .collect();

                    Ok(CacheQueryResult::GotCategory(topics_with_user))
                }

                CacheQuery::UpdateCategory(topics) => {
                    let category_id = topics[0].topic.category_id;
                    let category_key = format!("category:{}", &category_id);

                    let mut topic_rank_vec: Vec<(u32, String)> = Vec::with_capacity(20);
                    let mut user_rank_vec: Vec<(u32, String)> = Vec::with_capacity(20);
                    for topic in topics.iter() {
                        if let Some(user_id) = topic.check_user_id() {
                            let tuple = (user_id, json::to_string(&topic.user).unwrap());
                            if !user_rank_vec.contains(&tuple) {
                                user_rank_vec.push(tuple)
                            }
                        }
                        topic_rank_vec
                            .push((topic.get_self_id_copy(), json::to_string(&topic).unwrap()));
                    }

                    conn.zadd_multiple("users", &user_rank_vec)?;
                    conn.zadd_multiple(category_key, &topic_rank_vec)?;

                    Ok(CacheQueryResult::Updated)
                }

                CacheQuery::UpdateTopic(topic_with_post) => {
                    let mut topic_rank_vec: Vec<(u32, String)> = Vec::with_capacity(1);
                    let mut post_rank_vec: Vec<(u32, String)> = Vec::with_capacity(21);
                    let mut user_rank_vec: Vec<(u32, String)> = Vec::with_capacity(21);

                    if let Some(topic) = &topic_with_post.topic {
                        let topic_string = json::to_string(&topic.topic)?;
                        topic_rank_vec.push((
                            topic.get_self_id().clone(),
                            topic_string,
                        ));
                        if let Some(user_id) = topic.check_user_id() {
                            let user_string = json::to_string(&topic.user)?;
                            user_rank_vec.push((user_id, user_string));
                        }
                    }
                    if let Some(posts_with_user) = &topic_with_post.posts {
                        for post_with_user in posts_with_user.iter() {
                            if let Some(user_id) = post_with_user.check_user_id() {
                                let user_string = json::to_string(&post_with_user.user)?;
                                let tuple = (user_id, user_string);
                                if !user_rank_vec.contains(&tuple) {
                                    user_rank_vec.push(tuple)
                                }
                            }
                            let post_string = json::to_string(&post_with_user.post)?;
                            post_rank_vec.push((
                                post_with_user.get_self_id().clone(),
                                post_string,
                            ));
                        }
                    }
                    if !topic_rank_vec.is_empty() {
                        let category_key =
                            format!("category:{}", topic_with_post.get_category_id().unwrap());
                        conn.zadd_multiple(category_key,&topic_rank_vec)?;
                    }
                    if !user_rank_vec.is_empty() {
                        conn.zadd_multiple("users", &user_rank_vec)?;
                    }
                    if !post_rank_vec.is_empty() {
                        let topic_key =
                            format!("topic:{}", topic_with_post.get_topic_id().unwrap());
                        conn.zadd_multiple(topic_key, &post_rank_vec)?;
                    }
                    Ok(CacheQueryResult::Updated)
                }

//				CacheQuery::GetAllCategories => Ok(CacheQueryResult::GotAllCategories),
//
//				CacheQuery::GetPopular(_page) => Ok(CacheQueryResult::GotPopular),
            }
        }
    }
}

fn get_users<T>(vec: &Vec<T>, conn: &redis::Connection) -> Result<Vec<SlimUser>, ServiceError>
    where T: MatchUser {
    let mut user_id_vec = Vec::with_capacity(20);

    for item in vec.iter() {
        if !user_id_vec.contains(&item.get_user_id()) {
            user_id_vec.push(item.get_user_id())
        }
    }
    if user_id_vec.is_empty() { return Ok(vec![])}

    user_id_vec.sort();

    let range_index = user_id_vec.len() - 1;
    let range_start = user_id_vec[0].clone();
    let range_end = user_id_vec[range_index].clone();

    let users_vec: Vec<(String, u32)> = conn.zrangebyscore_withscores("users", range_start, range_end)?;

    let mut users: Vec<SlimUser> = Vec::with_capacity(20);
    for _user in users_vec.iter() {
        let (_user_string, _user_id) = _user;
        if user_id_vec.contains(&_user_id) {
            let usr = json::from_str(_user_string)?;
            users.push(usr)
        }
    };
    Ok(users)
}

pub fn match_cache_query_result(
    result: Result<CacheQueryResult, ServiceError>,
) -> Result<HttpResponse, ServiceError> {
    match result {
        Ok(query_result) => match query_result {
            CacheQueryResult::GotTopic(topic) => Ok(HttpResponse::Ok().json(topic)),
            CacheQueryResult::GotCategory(category_data) => Ok(HttpResponse::Ok().json(category_data)),
            _ => Ok(HttpResponse::Ok().finish()),
        },
        Err(e) => Err(e),
    }
}
