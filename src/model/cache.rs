use actix_web::HttpResponse;
use chrono::NaiveDateTime;

use crate::model::{
    errors::ServiceError,
    user::User,
    topic::Topic,
    post::Post,
};

pub trait SortHash<'a> {
    type Output;
    fn sort_hash(&'a self) -> Self::Output;
}

impl<'a> SortHash<'a> for Topic {
    type Output = Result<[(&'a str, String); 11], ServiceError>;

    fn sort_hash(&'a self) -> Result<[(&'a str, String); 11], ServiceError>
    {
        Ok([("id", serde_json::to_string(&self.id)?),
            ("user_id", serde_json::to_string(&self.user_id)?),
            ("category_id", serde_json::to_string(&self.category_id)?),
            ("title", serde_json::to_string(&self.title)?),
            ("body", serde_json::to_string(&self.body)?),
            ("thumbnail", serde_json::to_string(&self.thumbnail)?),
            ("created_at", serde_json::to_string(&self.created_at)?),
            ("updated_at", serde_json::to_string(&self.updated_at)?),
            ("last_reply_time", serde_json::to_string(&self.last_reply_time)?),
            ("reply_count", serde_json::to_string(&self.reply_count)?),
            ("is_locked", serde_json::to_string(&self.is_locked)?)])
    }
}

impl<'a> SortHash<'a> for User {
    type Output = Result<(u32, String), ServiceError>;
    fn sort_hash(&self) -> Result<(u32, String), ServiceError> {
        Ok((self.id, serde_json::to_string(&self)?))
    }
}


impl<'a> SortHash<'a> for Post {
    type Output = Result<[(&'a str, String); 10], ServiceError>;

    fn sort_hash(&'a self) -> Result<[(&'a str, String); 10], ServiceError>
    {
        Ok([("id", serde_json::to_string(&self.id)?),
            ("user_id", serde_json::to_string(&self.user_id)?),
            ("topic_id", serde_json::to_string(&self.topic_id)?),
            ("post_id", serde_json::to_string(&self.post_id)?),
            ("post_content", serde_json::to_string(&self.post_content)?),
            ("created_at", serde_json::to_string(&self.created_at)?),
            ("updated_at", serde_json::to_string(&self.updated_at)?),
            ("last_reply_time", serde_json::to_string(&self.last_reply_time)?),
            ("reply_count", serde_json::to_string(&self.reply_count)?),
            ("is_locked", serde_json::to_string(&self.is_locked)?),
        ])
    }
}

