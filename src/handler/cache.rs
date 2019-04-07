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
                    let category_key = format!("category:{}", 1);
                    let topic_string_vec: Vec<String> = redis::cmd("zrangebyscore")
                        .arg(&category_key)
                        .arg(topic_id.clone())
                        .arg(topic_id.clone())
                        .query(conn.deref())?;

                    if topic_string_vec.is_empty() { return Err(ServiceError::NotFound); };

                    let _topic: Topic = json::from_str(&topic_string_vec[0])?;

                    // collect unique user_ids
                    let mut user_id_vec: Vec<&u32> = Vec::with_capacity(21);
                    user_id_vec.push(_topic.get_user_id());

                    //get posts from redis. need to improve the code to be parallel with topic query
                    let page = cache_request.page;
                    let offset = (page - 1) * 20;
                    let topic_key = format!("topic:{}", &topic_id);

                    let post_string_vec: Vec<String> = redis::cmd("zrange")
                        .arg(&topic_key)
                        .arg(offset)
                        .arg(offset + LIMIT)
                        .query(conn.deref())?;

                    let mut post_vec: Vec<Post> = Vec::with_capacity(20);

                    for post_string in post_string_vec.iter() {
                        let _post: Post = json::from_str(&post_string)?;
                        post_vec.push(_post);
                    }

                    for post in post_vec.iter() {
                        if !user_id_vec.contains(&post.get_user_id()) {
                            user_id_vec.push(post.get_user_id())
                        }
                    }
                    user_id_vec.sort();

                    let range_index = user_id_vec.len() - 1;
                    let range_start = user_id_vec[0].clone();
                    let range_end = user_id_vec[range_index].clone();

                    let user_vec: Vec<(String, u32)> = conn.zrangebyscore_withscores("users", range_start, range_end)?;

                    let mut users: Vec<SlimUser> = Vec::with_capacity(21);
                    for _user in user_vec.iter() {
                        let (_user_string, _user_id) = _user;
                        if user_id_vec.contains(&_user_id) {
                            let usr = json::from_str(_user_string)?;
                            users.push(usr)
                        }
                    };

                    let posts = Some(post_vec.into_iter().map(|post| post.attach_user(&users)).collect());

                    let topic_with_post = if page == &1isize {
                        TopicWithPost {
                            topic: Some(_topic.attach_user(&users)),
                            posts,
                        }
                    } else {
                        TopicWithPost {
                            topic: None,
                            posts,
                        }
                    };
                    Ok(CacheQueryResult::GotTopic(topic_with_post))
                }

                CacheQuery::GetCategory(cache_request) => {
                    let page = cache_request.page;
                    let categories = cache_request.categories;

                    let offset = (page - 1) * 20;
                    let category_key = format!("category:{}", categories[0]);
                    let topics: Vec<String> = redis::cmd("zrevrange")
                        .arg(category_key)
                        .arg(offset)
                        .arg(offset + LIMIT)
                        .query(conn.deref())?;

                    if topics.len() == 0 {
                        return Err(ServiceError::NotFound);
                    }

                    let users: Vec<String> = redis::cmd("zrange")
                        .arg("users")
                        .arg(offset)
                        .arg(offset + LIMIT)
                        .query(conn.deref())?;

                    let _topics: Vec<Topic> = topics
                        .iter()
                        .map(|topic| json::from_str(&topic).unwrap())
                        .collect();
                    let _users: Vec<SlimUser> = users
                        .iter()
                        .map(|user| json::from_str(&user).unwrap())
                        .collect();

                    Ok(CacheQueryResult::GotCategory(
                        _topics
                            .into_iter()
                            .map(|topic| topic.attach_user(&_users))
                            .collect(),
                    ))
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
                                post_with_user.get_self_id_copy(),
                                post_string,
                            ));
                        }
                    }
                    if !topic_rank_vec.is_empty() {
                        let category_key =
                            format!("category:{}", topic_with_post.get_category_id().unwrap());
                        conn.zadd_multiple(category_key, &topic_rank_vec)?;
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
