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
            ("privilege", self.privilege.to_string()),
            ("show_email", self.show_email.to_string())]
    }
}

impl SortHash for Post {
    fn sort_hash(&self) -> Vec<(&str, String)> {
        vec![("id", self.id.to_string()),
             ("user_id", self.user_id.to_string()),
             ("topic_id", self.topic_id.to_string()),
             ("category_id", self.category_id.to_string()),
             ("post_id", self.post_id.unwrap_or(0).to_string()),
             ("post_content", self.post_content.to_owned()),
             ("created_at", self.created_at.to_string()),
             ("updated_at", self.updated_at.to_string()),
             ("is_locked", self.is_locked.to_string())]
    }
}

impl SortHash for Category {
    fn sort_hash(&self) -> Vec<(&str, String)> {
        vec![("id", self.id.to_string()),
             ("name", self.name.to_owned()),
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
        Ok(self.get(key).ok_or(ServiceError::InternalServerError)?.to_owned())
    }
    fn parse_date(&self, key: &str) -> Result<NaiveDateTime, ServiceError> {
        Ok(NaiveDateTime::parse_from_str(self.get(key).ok_or(ServiceError::InternalServerError)?, "%Y-%m-%d %H:%M:%S%.f")?)
    }
    fn parse_other<K>(&self, key: &str) -> Result<K, ServiceError>
        where K: FromStr {
        self.get(key).ok_or(ServiceError::InternalServerError)?.parse::<K>().map_err(|_| ServiceError::PARSE)
    }
    fn parse<X: FromHashSet>(&self) -> Result<X, ServiceError> {
        FromHashSet::from_hash(self)
    }
}

pub trait ParserMulti {
    fn skip(&self) -> Result<(), ServiceError>;
    fn parse_string(&self, key: &str) -> Result<String, ServiceError>;
    fn parse_date(&self, key: &str) -> Result<NaiveDateTime, ServiceError>;
    fn parse_other<K: FromStr>(&self, key: &str) -> Result<K, ServiceError>;
    fn parse_other_perm<K: FromStr>(&self, key: &str) -> Result<K, ServiceError>;
    fn parse<X: FromHashSetMulti>(&self) -> Result<X, ServiceError>;
}

impl ParserMulti for (HashMap<String, String>, HashMap<String, String>) {
    fn skip(&self) -> Result<(), ServiceError> {
        if self.0.is_empty() { Err(ServiceError::InternalServerError) } else { Ok(()) }
    }
    fn parse_string(&self, key: &str) -> Result<String, ServiceError> {
        Ok(self.0.get(key).ok_or(ServiceError::InternalServerError)?.to_owned())
    }
    fn parse_date(&self, key: &str) -> Result<NaiveDateTime, ServiceError> {
        Ok(NaiveDateTime::parse_from_str(self.0.get(key).ok_or(ServiceError::InternalServerError)?, "%Y-%m-%d %H:%M:%S%.f")?)
    }
    fn parse_other<K>(&self, key: &str) -> Result<K, ServiceError>
        where K: FromStr {
        self.0.get(key).ok_or(ServiceError::InternalServerError)?.parse::<K>().map_err(|_| ServiceError::PARSE)
    }
    fn parse_other_perm<K>(&self, key: &str) -> Result<K, ServiceError>
        where K: FromStr {
        self.1.get(key).ok_or(ServiceError::InternalServerError)?.parse::<K>().map_err(|_| ServiceError::PARSE)
    }
    fn parse<X: FromHashSetMulti>(&self) -> Result<X, ServiceError> {
        FromHashSetMulti::from_hash(self)
    }
}

pub trait FromHashSetMulti
    where Self: Sized {
    fn from_hash(hash: &(HashMap<String, String>, HashMap<String, String>)) -> Result<Self, ServiceError>;
}

impl FromHashSetMulti for Topic {
    fn from_hash(hash: &(HashMap<String, String>, HashMap<String, String>)) -> Result<Topic, ServiceError> {
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
            last_reply_time: hash.parse_date("last_reply_time").ok(),
            is_locked: hash.parse_other::<bool>("is_locked")?,
            reply_count: hash.parse_other_perm::<u32>("reply_count").ok(),
        })
    }
}

impl FromHashSetMulti for Post {
    fn from_hash(hash: &(HashMap<String, String>, HashMap<String, String>)) -> Result<Post, ServiceError> {
        hash.skip()?;
        let post_id = match hash.parse_other::<u32>("post_id").ok() {
            Some(id) => if id == 0 { None } else { Some(id) },
            None => None,
        };
        Ok(Post {
            id: hash.parse_other::<u32>("id")?,
            user_id: hash.parse_other::<u32>("user_id")?,
            topic_id: hash.parse_other::<u32>("topic_id")?,
            category_id: hash.parse_other::<u32>("category_id")?,
            post_id,
            post_content: hash.parse_string("post_content")?,
            created_at: hash.parse_date("created_at")?,
            updated_at: hash.parse_date("updated_at")?,
            last_reply_time: hash.parse_date("last_reply_time").ok(),
            is_locked: hash.parse_other::<bool>("is_locked")?,
            reply_count: hash.parse_other_perm::<u32>("reply_count").ok(),
        })
    }
}

pub trait FromHashSet
    where Self: Sized {
    fn from_hash(hash: &HashMap<String, String>) -> Result<Self, ServiceError>;
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
            privilege: hash.parse_other::<u32>("privilege")?,
            show_email: hash.parse_other::<bool>("show_email")?,
            online_status: hash.parse_other::<u32>("online_status").ok(),
            last_online: hash.parse_date("last_online").ok(),
        })
    }
}

impl FromHashSet for Category {
    fn from_hash(hash: &HashMap<String, String>) -> Result<Category, ServiceError> {
        hash.skip()?;
        Ok(Category {
            id: hash.parse_other::<u32>("id")?,
            name: hash.parse_string("name")?,
            thumbnail: hash.parse_string("thumbnail")?,
            topic_count: hash.parse_other::<u32>("topic_count").ok(),
            post_count: hash.parse_other::<u32>("post_count").ok(),
            topic_count_new: hash.parse_other::<u32>("topic_count_new").ok(),
            post_count_new: hash.parse_other::<u32>("post_count_new").ok(),
        })
    }
}

impl FromHashSet for Mail {
    fn from_hash(hash: &HashMap<String, String>) -> Result<Mail, ServiceError> {
        hash.skip()?;
        Ok(Mail {
            user_id: hash.parse_other::<u32>("user_id")?,
            username: "".to_string(),
            uuid: hash.parse_string("uuid")?,
            address: "".to_string(),
        })
    }
}