use actix::Message;
use chrono::NaiveDateTime;

use crate::schema::topics;
use crate::model::post::PostWithSlimUser;
use crate::model::user::{SlimUser, SlimmerUser};
use crate::model::errors::ServiceError;

#[derive(Debug, Identifiable, Queryable, Serialize)]
#[table_name = "topics"]
pub struct Topic {
    pub id: i32,
    pub user_id: i32,
    pub category_id: i32,
    pub title: String,
    pub body: String,
    pub thumbnail: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub last_reply_time: NaiveDateTime,
    pub reply_count: i32,
    pub is_locked: bool,
}

#[derive(Insertable)]
#[table_name = "topics"]
pub struct NewTopic {
    pub user_id: i32,
    pub category_id: i32,
    pub thumbnail: String,
    pub title: String,
    pub body: String,
}

#[derive(Deserialize)]
pub struct TopicRequest {
    pub category_id: i32,
    pub thumbnail: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct TopicResponse {
    pub topic: Option<TopicWithSlimUser>,
    pub posts: Option<Vec<PostWithSlimUser>>,
}

#[derive(Debug, Serialize)]
pub struct TopicWithSlimUser {
    pub id: i32,
    pub user: Option<SlimUser>,
    pub category_id: i32,
    pub title: String,
    pub body: String,
    pub thumbnail: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub last_reply_time: NaiveDateTime,
    pub reply_count: i32,
    pub is_locked: bool,
}

#[derive(Debug, Serialize)]
pub struct TopicWithSlimmerUser {
    pub id: i32,
    pub user: Option<SlimmerUser>,
    pub category_id: i32,
    pub title: String,
    pub body: String,
    pub thumbnail: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub last_reply_time: NaiveDateTime,
    pub reply_count: i32,
    pub is_locked: bool,
}

impl Topic {
    pub fn attach_slim_user(self, users: &Vec<SlimUser>) -> TopicWithSlimUser {
        let mut _index: Vec<usize> = Vec::with_capacity(1);
        for (index, user) in users.iter().enumerate() {
            if &self.user_id == &user.id {
                _index.push(index);
                break;
            }
        };
        if _index.len() == 0 {
            return  TopicWithSlimUser {
                id: self.id,
                user: None,
                category_id: self.category_id,
                title: self.title,
                body: self.body,
                thumbnail: self.thumbnail,
                created_at: self.created_at,
                updated_at: self.updated_at,
                last_reply_time: self.last_reply_time,
                reply_count: self.reply_count,
                is_locked: self.is_locked,
            }
        }
        TopicWithSlimUser {
            id: self.id,
            user: Some(users[_index[0]].clone()),
            category_id: self.category_id,
            title: self.title,
            body: self.body,
            thumbnail: self.thumbnail,
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_reply_time: self.last_reply_time,
            reply_count: self.reply_count,
            is_locked: self.is_locked,
        }
    }

    pub fn attach_slimmer_user(self, users: &Vec<SlimmerUser>) -> TopicWithSlimmerUser {
        let mut _index: Vec<usize> = Vec::with_capacity(1);
        for (index, user) in users.iter().enumerate() {
            if &self.user_id == &user.id {
                _index.push(index);
                break;
            }
        };
        if _index.len() == 0 {
            return TopicWithSlimmerUser {
                id: self.id,
                user: None,
                category_id: self.category_id,
                title: self.title,
                body: self.body,
                thumbnail: self.thumbnail,
                created_at: self.created_at,
                updated_at: self.updated_at,
                last_reply_time: self.last_reply_time,
                reply_count: self.reply_count,
                is_locked: self.is_locked,
            };
        }
        TopicWithSlimmerUser {
            id: self.id,
            user: Some(users[_index[0]].clone()),
            category_id: self.category_id,
            title: self.title,
            body: self.body,
            thumbnail: self.thumbnail,
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_reply_time: self.last_reply_time,
            reply_count: self.reply_count,
            is_locked: self.is_locked,
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct TopicUpdateRequest {
    pub id: Option<i32>,
    pub user_id: Option<i32>,
    pub category_id: Option<i32>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub thumbnail: Option<String>,
    pub last_reply_time: Option<bool>,
    pub is_locked: Option<bool>,
    pub is_admin: Option<bool>,
}

impl TopicUpdateRequest {
    pub fn update_topic_data(self, mut topic: Topic) -> Result<Topic, ()> {
        if let Some(new_category_id) = self.category_id {
            topic.category_id = new_category_id
        };
        if let Some(new_title) = self.title {
            topic.title = new_title
        };
        if let Some(new_body) = self.body {
            topic.body = new_body
        };
        if let Some(new_thumbnail) = self.thumbnail {
            topic.thumbnail = new_thumbnail
        };
        if let Some(bool) = self.last_reply_time {
            if bool == true {
                topic.last_reply_time = NaiveDateTime::parse_from_str("1970-01-01 23:33:33", "%Y-%m-%d %H:%M:%S").unwrap()
            }
        };
        if let Some(new_is_locked) = self.is_locked {
            topic.is_locked = new_is_locked
        };
        Ok(topic)
    }
}

impl Message for TopicQuery {
    type Result = Result<TopicQueryResult, ServiceError>;
}

pub enum TopicQuery {
    AddTopic(NewTopic),
    GetTopic(i32, i64),
    UpdateTopic(TopicUpdateRequest),
}

pub enum TopicQueryResult {
    AddedTopic,
    GotTopic(TopicResponse),
}

impl TopicQueryResult {
    pub fn to_topic_data(self) -> Option<TopicResponse> {
        match self {
            TopicQueryResult::GotTopic(topic_data) => Some(topic_data),
            _ => None
        }
    }
}