use actix_web::HttpResponse;
use chrono::NaiveDateTime;

use crate::model::{
    errors::ServiceError,
    user::PublicUser,
    topic::{TopicWithPost, TopicWithUser},
    common::ResponseMessage,
};
use crate::model::topic::TopicWithUser;

pub struct CategoryCacheRequest<'a> {
    pub categories: &'a Vec<u32>,
    pub page: &'a isize,
}

pub struct TopicCacheRequest<'a> {
    pub topic: &'a u32,
    pub page: &'a isize,
}

pub enum CacheQuery<'a> {
    //    GetAllCategories,
//    GetPopular(i64),
    GetTopic(TopicCacheRequest<'a>),
    GetCategory(CategoryCacheRequest<'a>),
    UpdateCategory(&'a Vec<TopicWithUser>),
    UpdateTopic(&'a TopicWithPost),
}

pub enum CacheQueryResult<'a> {
    //    GotAllCategories,
    GotPopular,
    Updated,
    GotCategory(&'a Vec<TopicWithUser<'a>>),
    GotTopic(TopicWithPost),
}

impl<'a> CacheQueryResult<'a> {
    pub fn to_response(&self) -> HttpResponse {
        match self {
            CacheQueryResult::GotCategory(categories) => HttpResponse::Ok().json(&categories),
            CacheQueryResult::GotTopic(topics) => HttpResponse::Ok().json(&topics),
            CacheQueryResult::Updated => HttpResponse::Ok().json(ResponseMessage::new("Modify Success")),
            CacheQueryResult::GotPopular => HttpResponse::Ok().json(ResponseMessage::new("Placeholder response")),
        }
    }
}