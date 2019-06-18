use std::collections::HashMap;
use std::str::FromStr;

use chrono::NaiveDateTime;

use crate::model::{
    category::Category,
    errors::ServiceError,
    post::Post,
    topic::Topic,
    user::User,
    mail::Mail,
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
            ("show_updated_at", self.show_updated_at.to_string())]
    }
}

impl SortHash for Post {
    fn sort_hash(&self) -> Vec<(&str, String)> {
        vec![("id", self.id.to_string()),
             ("user_id", self.user_id.to_string()),
             ("topic_id", self.topic_id.to_string()),
             ("post_id", self.post_id.unwrap_or(0).to_string()),
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
            ("thumbnail", self.thumbnail.to_owned())]
    }
}

impl SortHash for Mail {
    fn sort_hash(&self) -> Vec<(&str, String)> {
        vec![
            ("user_id", self.user_id.to_string()),
            ("uuid", self.uuid.to_owned())]
    }
}

pub trait Parser {
    fn skip(&self) -> Result<(), ServiceError>;
    fn parse_string(&self, key: &str) -> Result<String, ServiceError>;
    fn parse_date(&self, key: &str) -> Result<NaiveDateTime, ServiceError>;
    fn parse_other<K: FromStr>(&self, key: &str) -> Result<K, ServiceError>;

    fn parse<X: FromHashSet>(&self) -> Result<X, ServiceError>;
}

impl Parser for HashMap<String, String> {
    fn skip(&self) -> Result<(), ServiceError> {
        if self.is_empty() { Err(ServiceError::InternalServerError) } else { Ok(()) }
    }
    fn parse_string(&self, key: &str) -> Result<String, ServiceError> {
        self.get(key).map(|s|s.to_owned()).ok_or(ServiceError::InternalServerError)
    }
    fn parse_date(&self, key: &str) -> Result<NaiveDateTime, ServiceError> {
        Ok(self.get(key).map(|s|NaiveDateTime::parse_from_str(s,"%Y-%m-%d %H:%M:%S%.f")).unwrap()?)
    }
    fn parse_other<K>(&self, key: &str) -> Result<K, ServiceError>
        where K: FromStr {
        self.get(key).map(|s|s.parse::<K>().map_err(|_| ServiceError::InternalServerError)).unwrap()
    }
    fn parse<X: FromHashSet>(&self) -> Result<X, ServiceError> {
        FromHashSet::from_hash(self)
    }
}

pub trait FromHashSet
    where Self: Sized {
    fn from_hash(hash: &HashMap<String, String>) -> Result<Self, ServiceError>;
}

impl FromHashSet for Post {
    fn from_hash(hash: &HashMap<String, String>) -> Result<Post, ServiceError> {
        hash.skip()?;
        let post_id = match hash.parse_other::<u32>("post_id").ok() {
            Some(id) => if id == 0 { None } else { Some(id) },
            None => None,
        };
        Ok(Post {
            id: hash.parse_other::<u32>("id")?,
            user_id: hash.parse_other::<u32>("user_id")?,
            topic_id: hash.parse_other::<u32>("topic_id")?,
            post_id,
            post_content: hash.parse_string("post_content")?,
            created_at: hash.parse_date("created_at")?,
            updated_at: hash.parse_date("updated_at")?,
            last_reply_time: hash.parse_date("last_reply_time")?,
            reply_count: hash.parse_other::<i32>("reply_count")?,
            is_locked: hash.parse_other::<bool>("is_locked")?,
        })
    }
}

impl FromHashSet for Topic {
    fn from_hash(hash: &HashMap<String, String>) -> Result<Topic, ServiceError> {
        hash.skip()?;
        Ok(Topic {
            id: hash.parse_other::<u32>("id")?,
            user_id: hash.parse_other::<u32>("user_id")?,
            category_id: hash.parse_other::<u32>("category_id")?,
            title: hash.parse_string("title")?,
            body: hash.parse_string("body")?,
            thumbnail: hash.parse_string("thumbnail")?,
            created_at: hash.parse_date("created_at")?,
            updated_at: hash.parse_date("updated_at")?,
            last_reply_time: hash.parse_date("last_reply_time")?,
            reply_count: hash.parse_other::<i32>("reply_count")?,
            is_locked: hash.parse_other::<bool>("is_locked")?,
        })
    }
}

impl FromHashSet for User {
    fn from_hash(hash: &HashMap<String, String>) -> Result<User, ServiceError> {
        hash.skip()?;
        Ok(User {
            id: hash.parse_other::<u32>("id")?,
            username: hash.parse_string("username")?,
            email: hash.parse_string("email")?,
            hashed_password: "".to_string(),
            avatar_url: hash.parse_string("avatar_url")?,
            signature: hash.parse_string("signature")?,
            created_at: hash.parse_date("created_at")?,
            updated_at: hash.parse_date("updated_at")?,
            is_admin: hash.parse_other::<u32>("is_admin")?,
            blocked: hash.parse_other::<bool>("blocked")?,
            show_email: hash.parse_other::<bool>("show_email")?,
            show_created_at: hash.parse_other::<bool>("show_created_at")?,
            show_updated_at: hash.parse_other::<bool>("show_updated_at")?,
        })
    }
}

impl FromHashSet for Category {
    fn from_hash(hash: &HashMap<String, String>) -> Result<Category, ServiceError> {
        hash.skip()?;
        Ok(Category {
            id: hash.parse_other::<u32>("id")?,
            name: hash.parse_string("name")?,
            topic_count: hash.parse_other::<i32>("topic_count")?,
            post_count: hash.parse_other::<i32>("post_count")?,
            subscriber_count: hash.parse_other::<i32>("subscriber_count")?,
            thumbnail: hash.parse_string("thumbnail")?,
        })
    }
}

pub enum CacheQuery {
    GetUser(u32),
    GetPost(u32),
    GetTopic(u32, i64),
    GetTopics(Vec<u32>, i64),
    GetAllCategories,
}

pub trait IdToUserQuery {
    fn to_query_cache(&self) -> CacheQuery;
}

impl IdToUserQuery for u32 {
    fn to_query_cache(&self) -> CacheQuery { CacheQuery::GetUser(*self) }
}

pub trait IdToPostQuery {
    fn to_query_cache(&self) -> CacheQuery;
}

impl IdToPostQuery for u32 {
    fn to_query_cache(&self) -> CacheQuery { CacheQuery::GetPost(*self) }
}

pub trait PathToTopicQuery {
    fn to_query_cache(&self) -> CacheQuery;
}

impl PathToTopicQuery for (u32, i64) {
    fn to_query_cache(&self) -> CacheQuery { CacheQuery::GetTopic(self.0, self.1) }
}

pub trait PathToTopicsQuery {
    fn to_query_cache(&self) -> CacheQuery;
}

impl PathToTopicsQuery for (u32, i64) {
    fn to_query_cache(&self) -> CacheQuery { CacheQuery::GetTopics(vec![self.0], self.1) }
}

pub trait PathToCategoryQuery {
    fn to_query_cache(&self) -> CacheQuery;
}

impl PathToCategoryQuery for (u32, i64) {
    fn to_query_cache(&self) -> CacheQuery { CacheQuery::GetTopics(vec![self.0], self.1) }
}