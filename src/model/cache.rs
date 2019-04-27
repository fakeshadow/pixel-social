use actix_web::HttpResponse;
use chrono::NaiveDateTime;

use crate::model::common::GetSelfId;

#[derive(Debug)]
pub struct TopicHashSet<'a> {
    pub id: &'a u32,
    pub user_id: &'a u32,
    pub category_id: &'a u32,
    pub created_at: &'a NaiveDateTime,
    pub updated_at: &'a NaiveDateTime,
    pub last_reply_time: &'a NaiveDateTime,
    pub reply_count: &'a i32,
    pub is_locked: &'a bool,
}

#[derive(Serialize, Debug)]
pub struct TopicRankSet<'a> {
    pub id: &'a u32,
    pub title: &'a str,
    pub body: &'a str,
    pub thumbnail: &'a str,
}

impl<'a> GetSelfId for TopicRankSet<'a> {
    fn get_self_id(&self) -> &u32 { &self.id }
}