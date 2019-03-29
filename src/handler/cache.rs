use actix::Handler;

use crate::model::errors::ServiceError;
use crate::model::{db::CacheExecutor, topic::*, user::*};
use crate::model::cache::{CacheQuery, CacheQueryResult};

use serde_json;
use redis::{Commands};

const LIMIT: isize = 20;

impl Handler<CacheQuery> for CacheExecutor {
    type Result = Result<CacheQueryResult, ServiceError>;

    fn handle(&mut self, message: CacheQuery, _: &mut Self::Context) -> Self::Result {
        let conn = &self.0;

        match message {
            CacheQuery::GetCategory(cache_request) => {
                let category_id = cache_request.categories.unwrap_or(vec![1]);
                let page = cache_request.page.unwrap_or(1);
                let offset = (page - 1) * 20;

                let redis = conn.get_connection()?;

                let category_data: Vec<String> = redis.zrange("test123", offset, offset + LIMIT)?;

                let topics: Vec<TopicWithUser<SlimmerUser>> = category_data
                    .iter()
                    .map(|topic| {
                        serde_json::from_str(topic).unwrap()
                    })
                    .collect();

                Ok(CacheQueryResult::GotCategory(topics))
            }
        }
    }
}
