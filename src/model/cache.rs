use actix_web::HttpResponse;
use chrono::NaiveDateTime;

use crate::model::{
    errors::ServiceError,
    user::User,
    topic::Topic,
    post::Post,
    category::Category,
};
use std::collections::HashMap;
use crate::handler::cache::UpdateCache::Categories;

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

pub trait FromHashMap<T, P, C> {
    fn map_topic(&self) -> Result<T, ServiceError>;
    fn map_post(&self) -> Result<P, ServiceError>;
    fn map_category(&self) -> Result<C, ServiceError>;
}

impl FromHashMap<Topic, Post, Category> for HashMap<String, String> {
    fn map_topic(&self) -> Result<Topic, ServiceError> {
        Ok(Topic {
            id: self.get("id").ok_or(ServiceError::InternalServerError)?.parse::<u32>().unwrap(),
            user_id: self.get("user_id").ok_or(ServiceError::InternalServerError)?.parse::<u32>().unwrap(),
            category_id: self.get("category_id").ok_or(ServiceError::InternalServerError)?.parse::<u32>().unwrap(),
            title: self.get("title").ok_or(ServiceError::InternalServerError)?.to_string(),
            body: self.get("body").ok_or(ServiceError::InternalServerError)?.to_string(),
            thumbnail: self.get("thumbnail").ok_or(ServiceError::InternalServerError)?.to_string(),
            created_at: NaiveDateTime::parse_from_str(self.get("created_at").ok_or(ServiceError::InternalServerError)?, "%Y-%m-%d %H:%M:%S%.f").unwrap(),
            updated_at: NaiveDateTime::parse_from_str(self.get("updated_at").ok_or(ServiceError::InternalServerError)?, "%Y-%m-%d %H:%M:%S%.f").unwrap(),
            last_reply_time: NaiveDateTime::parse_from_str(self.get("last_reply_time").ok_or(ServiceError::InternalServerError)?, "%Y-%m-%d %H:%M:%S%.f").unwrap(),
            reply_count: self.get("reply_count").ok_or(ServiceError::InternalServerError)?.parse::<i32>().unwrap(),
            is_locked: self.get("is_locked").ok_or(ServiceError::InternalServerError)?.parse::<bool>().unwrap(),
        })
    }

    fn map_post(&self) -> Result<Post, ServiceError> {
        let post_id = match self.get("post_id").ok_or(ServiceError::InternalServerError)?.parse::<u32>().ok() {
            Some(id) => if id == 0 { None } else { Some(id) },
            None => None,
        };
        Ok(Post {
            id: self.get("id").ok_or(ServiceError::InternalServerError)?.parse::<u32>().unwrap(),
            user_id: self.get("user_id").ok_or(ServiceError::InternalServerError)?.parse::<u32>().unwrap(),
            topic_id: self.get("topic_id").ok_or(ServiceError::InternalServerError)?.parse::<u32>().unwrap(),
            post_id,
            post_content: self.get("post_content").ok_or(ServiceError::InternalServerError)?.to_string(),
            created_at: NaiveDateTime::parse_from_str(self.get("created_at").ok_or(ServiceError::InternalServerError)?, "%Y-%m-%d %H:%M:%S%.f").unwrap(),
            updated_at: NaiveDateTime::parse_from_str(self.get("updated_at").ok_or(ServiceError::InternalServerError)?, "%Y-%m-%d %H:%M:%S%.f").unwrap(),
            last_reply_time: NaiveDateTime::parse_from_str(self.get("last_reply_time").ok_or(ServiceError::InternalServerError)?, "%Y-%m-%d %H:%M:%S%.f").unwrap(),
            reply_count: self.get("reply_count").ok_or(ServiceError::InternalServerError)?.parse::<i32>().unwrap(),
            is_locked: self.get("is_locked").ok_or(ServiceError::InternalServerError)?.parse::<bool>().unwrap(),
        })
    }

    fn map_category(&self) -> Result<Category, ServiceError> {
        Ok(Category {
            id: self.get("id").ok_or(ServiceError::InternalServerError)?.parse::<u32>().unwrap(),
            name: self.get("name").ok_or(ServiceError::InternalServerError)?.to_string(),
            topic_count: self.get("topic_count").ok_or(ServiceError::InternalServerError)?.parse::<u32>().unwrap(),
            post_count: self.get("post_count").ok_or(ServiceError::InternalServerError)?.parse::<u32>().unwrap(),
            subscriber_count: self.get("subscriber_count").ok_or(ServiceError::InternalServerError)?.parse::<u32>().unwrap(),
            thumbnail: self.get("thumbnail").ok_or(ServiceError::InternalServerError)?.to_string(),
        })
    }
}