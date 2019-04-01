use actix_web::{web, HttpResponse};
use serde_json;
use r2d2_redis::redis;
use r2d2_redis::redis::{Commands, RedisError};

use crate::model::{topic::*, user::*,common::*, errors::ServiceError, cache::{CacheQuery, CacheQueryResult}};

const LIMIT: isize = 20;

pub fn cache_handler(query: CacheQuery, pool: &web::Data<RedisPool>) -> Result<CacheQueryResult, ServiceError> {
    match &pool.try_get() {
        None => Err(ServiceError::RedisOffline),
        Some(conn) => {
            match query {
                CacheQuery::GetCategory(cache_request) => {
                    let page = cache_request.page;
                    let categories = cache_request.categories;

                    let offset = (page - 1) * 20;
                    let category_key = format!("category:{}", categories[0]);
                    use lazy_static::__Deref;
                    let result: Result<Vec<String>, RedisError> = redis::cmd("zrange")
                        .arg(category_key)
                        .arg(offset)
                        .arg(offset + LIMIT)
                        .query(conn.deref());

                    match result {
                        Ok(category_data) => {
                            if category_data.len() == 0 { return Err(ServiceError::NoCacheFound); }
                            let topics: Vec<TopicWithUser<SlimmerUser>> = category_data
                                .iter()
                                .map(|topic| {
                                    serde_json::from_str(topic).unwrap()
                                })
                                .collect();

                            Ok(CacheQueryResult::GotCategory(topics))
                        }
                        Err(_) => {
                            Err(ServiceError::NoCacheFound)
                        }
                    }
                }

                CacheQuery::UpdateCategory(topics) => {
                    let category_id = topics[0].topic.category_id;
                    let category_key = format!("category:{}", &category_id);

                    println!("updating cache");
                    let mut topic_rank_cache_vec = Vec::with_capacity(20);
                    for topic in topics.iter() {
// topic rank store topic title/body/thumbnail with topic_id as score with category_id as key
// topic hash store other fields
                        let topic_id = topic.get_self_id();
                        topic_rank_cache_vec.push((topic_id.clone(), serde_json::to_string(topic).unwrap()))
                    }

                    let _result: usize = conn.zadd_multiple(category_key, &topic_rank_cache_vec)?;

                    Ok(CacheQueryResult::GotCategory(topics))
                }

                CacheQuery::GetAllCategories => {
                    Ok(CacheQueryResult::GotAllCategories)
                }

                CacheQuery::GetPopular(page) => {
                    Ok(CacheQueryResult::GotPopular)
                }
            }
        }
    }
}

