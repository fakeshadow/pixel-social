
use actix_web::{web, Error, HttpResponse, ResponseError};
use crate::model::errors::ServiceError;

use crate::model::{topic::*, user::*};
use crate::model::cache::{CacheQuery, CacheQueryResult};

use serde_json;

use crate::model::common::GetSelfTimeStamp;

use r2d2_redis::{redis};
use r2d2_redis::redis::{Commands, RedisError};

use crate::model::types::*;

const LIMIT: isize = 20;

pub fn cache_handler(query: CacheQuery, pool: web::Data<RedisPool>) ->  Result<CacheQueryResult, ServiceError> {


    let conn = &pool.get().unwrap();

    match query {
        CacheQuery::GetCategory(cache_request) => {
            let category_id = cache_request.categories.unwrap_or(vec![1]);
            let category_key = format!("category:{}", &category_id[0]);
            let page = cache_request.page.unwrap_or(1);
            let offset = (page - 1) * 20;

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

            let mut cache_vec = Vec::with_capacity(20);
            for topic in topics.iter() {
                let timestamp = topic.get_last_reply_timestamp();
                cache_vec.push((timestamp, serde_json::to_string(topic).unwrap()))
            }

            let _result: usize = conn.zadd_multiple(category_key, &cache_vec)?;

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
