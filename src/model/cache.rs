use actix_web::HttpResponse;
use chrono::NaiveDateTime;

use crate::model::{
    topic::Topic,
    common::GetSelfId,
};
use crate::model::user::User;
use crate::model::errors::ServiceError;

pub trait SortHash<'a> {
    type Output;
    fn sort_hash(&'a self) -> Self::Output;
}

#[derive(Serialize, Debug)]
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

impl<'a> SortHash<'a> for TopicHashSet<'a> {
    type Output = Result<[(&'a str, String); 8], ServiceError>;

    fn sort_hash(&'a self) -> Result<[(&'a str, String); 8], ServiceError>
    {
        Ok([("id", serde_json::to_string(self.id)?),
            ("user_id", serde_json::to_string(self.user_id)?),
            ("category_id", serde_json::to_string(self.category_id)?),
            ("created_at", serde_json::to_string(self.created_at)?),
            ("updated_at", serde_json::to_string(self.updated_at)?),
            ("last_reply_time", serde_json::to_string(self.last_reply_time)?),
            ("reply_count", serde_json::to_string(self.reply_count)?),
            ("is_locked", serde_json::to_string(self.is_locked)?)])
    }
}

impl<'a> GetSelfId for TopicHashSet<'a> {
    fn get_self_id(&self) -> &u32 {self.id}
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


#[derive(Serialize, Debug)]
pub struct PostHashSet {
    pub id
}





#[derive(Serialize, Debug)]
pub struct UserRankSet<'a> {
    pub id: &'a u32,
    pub username: &'a str,
    pub email: Option<&'a str>,
    pub avatar_url: &'a str,
    pub signature: &'a str,
    pub created_at: Option<&'a NaiveDateTime>,
    pub updated_at: Option<&'a NaiveDateTime>,
    pub is_admin: &'a u32,
    pub blocked: &'a bool,
    pub show_email: &'a bool,
    pub show_created_at: &'a bool,
    pub show_updated_at: &'a bool,
}

