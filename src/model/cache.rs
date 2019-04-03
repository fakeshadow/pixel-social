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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopicHash {
    pub id: u32,
    pub user_id: u32,
    pub category_id: u32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub last_reply_time: NaiveDateTime,
    pub reply_count: u32,
    pub is_locked: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopicRank {
    pub score: u32,
    pub title: String,
    pub body: String,
    pub thumbnail: String,
}

pub type UserHash = SlimUser;

#[derive(Debug, Serialize, Deserialize)]
pub struct PostHash {
    pub id: i32,
    pub user_id: u32,
    pub topic_id: u32,
    pub post_id: Option<u32>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub last_reply_time: NaiveDateTime,
    pub reply_count: u32,
    pub is_locked: bool,
}

#[derive(Debug, Queryable, Serialize, Deserialize)]
pub struct PostRank {
    pub post_content: String,
}

#[derive(Debug, Queryable, Serialize, Deserialize)]
pub struct CategoryRank {
    pub name: String,
    pub theme: String,
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
