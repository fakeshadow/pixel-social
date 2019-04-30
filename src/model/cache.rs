use actix_web::HttpResponse;
use chrono::NaiveDateTime;

use crate::model::{
    errors::ServiceError,
    user::User,
    topic::Topic,
    post::Post,
    category::Category,
};

pub trait SortHash<'a, T> {
    fn sort_hash(&'a self) -> T;
}

impl<'a> SortHash<'a, [(&'a str, String); 11]> for Topic {
    fn sort_hash(&'a self) -> [(&'a str, String); 11] {
        [
            ("id", self.id.to_string()),
            ("user_id", self.user_id.to_string()),
            ("category_id", self.category_id.to_string()),
            ("title", self.title.to_owned()),
            ("body", self.body.to_owned()),
            ("thumbnail", self.thumbnail.to_owned()),
            ("created_at", self.created_at.to_string()),
            ("updated_at", self.updated_at.to_string()),
            ("last_reply_time", self.last_reply_time.to_string()),
            ("reply_count", self.reply_count.to_string()),
            ("is_locked", self.is_locked.to_string())]
    }
}

impl<'a> SortHash<'a, Result<(u32, String), ServiceError>> for User {
    fn sort_hash(&self) -> Result<(u32, String), ServiceError> {
        Ok((self.id, serde_json::to_string(&self)?))
    }
}

impl<'a> SortHash<'a, [(&'a str, String); 10]> for Post {
    fn sort_hash(&'a self) -> [(&'a str, String); 10] {
        let pid = match &self.post_id {
            Some(id) => id,
            None => &0
        };
        [
            ("id", self.id.to_string()),
            ("user_id", self.user_id.to_string()),
            ("topic_id", self.topic_id.to_string()),
            ("post_id", pid.to_string()),
            ("post_content", self.post_content.to_owned()),
            ("created_at", self.created_at.to_string()),
            ("updated_at", self.updated_at.to_string()),
            ("last_reply_time", self.last_reply_time.to_string()),
            ("reply_count", self.reply_count.to_string()),
            ("is_locked", self.is_locked.to_string())
        ]
    }
}

impl<'a> SortHash<'a, [(&'a str, String); 6]> for Category {
    fn sort_hash(&'a self) -> [(&'a str, String); 6] {
        [
            ("id", self.id.to_string()),
            ("name", self.name.to_owned()),
            ("topic_count", self.topic_count.to_string()),
            ("post_count", self.post_count.to_string()),
            ("subscriber_count", self.subscriber_count.to_string()),
            ("thumbnail", self.thumbnail.to_owned())
        ]
    }
}

