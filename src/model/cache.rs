use std::str::FromStr;
use std::collections::HashMap;

use chrono::NaiveDateTime;

use crate::model::{
    errors::ServiceError,
    user::User,
    topic::Topic,
    post::Post,
    category::Category,
};


// ToDo: add individual field sort
pub trait SortHash {
    fn sort_hash(&self) -> Vec<(&str, String)>;
}

impl SortHash for Topic {
    fn sort_hash(&self) -> Vec<(&str, String)> {
        vec![
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

impl SortHash for User {
    fn sort_hash(&self) -> Vec<(&str, String)> {
        vec![
            ("id", self.id.to_string()),
            ("username", self.username.to_owned()),
            ("email", self.email.to_string()),
            ("avatar_url", self.avatar_url.to_owned()),
            ("signature", self.signature.to_owned()),
            ("created_at", self.created_at.to_string()),
            ("updated_at", self.updated_at.to_string()),
            ("is_admin", self.is_admin.to_string()),
            ("blocked", self.blocked.to_string()),
            ("show_email", self.show_email.to_string()),
            ("show_created_at", self.show_created_at.to_string()),
            ("show_updated_at", self.show_updated_at.to_string()),
        ]
    }
}

impl SortHash for Post {
    fn sort_hash(&self) -> Vec<(&str, String)> {
        let pid = match &self.post_id {
            Some(id) => id,
            None => &0
        };
        vec![("id", self.id.to_string()),
             ("user_id", self.user_id.to_string()),
             ("topic_id", self.topic_id.to_string()),
             ("post_id", pid.to_string()),
             ("post_content", self.post_content.to_owned()),
             ("created_at", self.created_at.to_string()),
             ("updated_at", self.updated_at.to_string()),
             ("last_reply_time", self.last_reply_time.to_string()),
             ("reply_count", self.reply_count.to_string()),
             ("is_locked", self.is_locked.to_string())]
    }
}

impl SortHash for Category {
    fn sort_hash(&self) -> Vec<(&str, String)> {
        vec![
            ("id", self.id.to_string()),
            ("name", self.name.to_owned()),
            ("topic_count", self.topic_count.to_string()),
            ("post_count", self.post_count.to_string()),
            ("subscriber_count", self.subscriber_count.to_string()),
            ("thumbnail", self.thumbnail.to_owned())
        ]
    }
}

pub trait FromHashMap<T, P, C, U> {
    fn skip(&self) -> Result<(), ServiceError>;
    fn parse<K: FromStr>(&self, key: &str) -> Result<K, ServiceError>;
    fn parse_string(&self, key: &str) -> Result<String, ServiceError>;
    fn parse_date(&self, key: &str) -> Result<NaiveDateTime, ServiceError>;

    fn parse_topic(&self) -> Result<T, ServiceError>;
    fn parse_post(&self) -> Result<P, ServiceError>;
    fn parse_category(&self) -> Result<C, ServiceError>;
    fn parse_user(&self) -> Result<U, ServiceError>;
}

impl FromHashMap<Topic, Post, Category, User> for HashMap<String, String> {
    fn skip(&self) -> Result<(), ServiceError> {
        if self.is_empty() { Err(ServiceError::NoCacheFound) } else { Ok(()) }
    }
    fn parse<K>(&self, key: &str) -> Result<K, ServiceError>
        where K: FromStr {
        Ok(self.get(key).ok_or(ServiceError::InternalServerError)?
            .parse::<K>().map_err(|_| ServiceError::InternalServerError)?)
    }
    fn parse_string(&self, key: &str) -> Result<String, ServiceError> {
        Ok(self.get(key).ok_or(ServiceError::InternalServerError)?.to_string())
    }
    fn parse_date(&self, key: &str) -> Result<NaiveDateTime, ServiceError> {
        Ok(NaiveDateTime::parse_from_str(self.get(key).ok_or(ServiceError::InternalServerError)?, "%Y-%m-%d %H:%M:%S%.f")?)
    }

    fn parse_topic(&self) -> Result<Topic, ServiceError> {
        self.skip()?;
        Ok(Topic {
            id: self.parse::<u32>("id")?,
            user_id: self.parse::<u32>("user_id")?,
            category_id: self.parse::<u32>("category_id")?,
            title: self.parse_string("title")?,
            body: self.parse_string("body")?,
            thumbnail: self.parse_string("thumbnail")?,
            created_at: self.parse_date("created_at")?,
            updated_at: self.parse_date("updated_at")?,
            last_reply_time: self.parse_date("last_reply_time")?,
            reply_count: self.parse::<i32>("reply_count")?,
            is_locked: self.parse::<bool>("is_locked")?,
        })
    }

    fn parse_post(&self) -> Result<Post, ServiceError> {
        self.skip()?;
        // ToDo: remove this check
        let post_id = match self.parse::<u32>("post_id").ok() {
            Some(id) => if id == 0 { None } else { Some(id) },
            None => None,
        };
        Ok(Post {
            id: self.parse::<u32>("id")?,
            user_id: self.parse::<u32>("user_id")?,
            topic_id: self.parse::<u32>("topic_id")?,
            post_id,
            post_content: self.parse_string("post_content")?,
            created_at: self.parse_date("created_at")?,
            updated_at: self.parse_date("updated_at")?,
            last_reply_time: self.parse_date("last_reply_time")?,
            reply_count: self.parse::<i32>("reply_count")?,
            is_locked: self.parse::<bool>("is_locked")?,
        })
    }

    fn parse_category(&self) -> Result<Category, ServiceError> {
        self.skip()?;
        Ok(Category {
            id: self.parse::<u32>("id")?,
            name: self.parse_string("name")?,
            topic_count: self.parse::<i32>("topic_count")?,
            post_count: self.parse::<i32>("post_count")?,
            subscriber_count: self.parse::<i32>("subscriber_count")?,
            thumbnail: self.parse_string("thumbnail")?,
        })
    }

    fn parse_user(&self) -> Result<User, ServiceError> {
        self.skip()?;
        Ok(User {
            id: self.parse::<u32>("id")?,
            username: self.parse_string("username")?,
            email: self.parse_string("email")?,
            hashed_password: "".to_string(),
            avatar_url: self.parse_string("avatar_url")?,
            signature: self.parse_string("signature")?,
            created_at: self.parse_date("created_at")?,
            updated_at: self.parse_date("updated_at")?,
            is_admin: self.parse::<u32>("is_admin")?,
            blocked: self.parse::<bool>("blocked")?,
            show_email: self.parse::<bool>("show_email")?,
            show_created_at: self.parse::<bool>("show_created_at")?,
            show_updated_at: self.parse::<bool>("show_updated_at")?,
        }
        )
    }
}