use crate::model::errors::ServiceError;

use chrono::NaiveDateTime;

use crate::model::{topic::*, user::*};

pub struct CategoryCacheRequest<'a> {
    pub categories: &'a Vec<u32>,
    pub page: &'a isize,
}

pub struct TopicCacheRequest<'a> {
    pub topic: &'a u32,
    pub page: &'a isize,
}

pub enum CacheQuery<'a> {
    GetAllCategories,
    GetPopular(i64),
    GetTopic(TopicCacheRequest<'a>),
    GetCategory(CategoryCacheRequest<'a>),
    UpdateCategory(Vec<TopicWithUser<SlimUser>>),
    UpdateTopic(&'a TopicWithPost)
}

pub enum CacheQueryResult {
    GotAllCategories,
    GotPopular,
    GotCategory(Vec<TopicWithUser<SlimUser>>),
    GotTopic(TopicWithPost),
}
