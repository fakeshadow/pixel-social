use actix::Message;
use crate::model::errors::ServiceError;

use crate::model::{topic::*, user::*};

#[derive(Deserialize)]
pub struct CacheRequest {
    pub categories: Option<Vec<i32>>,
    pub page: Option<isize>,
}

impl Message for CacheQuery {
    type Result = Result<CacheQueryResult, ServiceError>;
}

pub enum CacheQuery {
    GetAllCategories,
    GetPopular(i64),
    GetCategory(CacheRequest),
    UpdateCategory(Vec<TopicWithUser<SlimmerUser>>)
}

pub enum CacheQueryResult {
    GotAllCategories,
    GotPopular,
    GotCategory(Vec<TopicWithUser<SlimmerUser>>),
    Tested(String)
}