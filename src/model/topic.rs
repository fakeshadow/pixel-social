use actix::Message;
use chrono::NaiveDateTime;
use crate::schema::topics;

use crate::model::errors::ServiceError;

#[derive(Identifiable, Queryable, Serialize)]
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

#[derive(Deserialize, Clone)]
pub struct TopicUpdateRequest {
    pub id: Option<i32>,
    pub category_id: Option<i32>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub thumbnail: Option<String>,
    pub last_reply_time: Option<bool>,
    pub is_locked: Option<bool>,
}

impl TopicUpdateRequest {
    pub fn update_topic_data(self, mut topic: Topic) -> Result<Topic, ()> {
        if let Some(new_username) = self.category_id {
            topic.category_id = new_username
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
                topic.last_reply_time = NaiveDateTime::parse_from_str("2015-09-05 23:56:04", "%Y-%m-%d %H:%M:%S").unwrap()
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
    GetTopic(i32),
    UpdateTopic(TopicUpdateRequest),
}

pub enum TopicQueryResult {
    AddedTopic,
    UpdatedTopic,
    GotTopic(Topic),
}

impl TopicQueryResult {
    pub fn to_topic_data(self) -> Option<Topic> {
        match self {
            TopicQueryResult::GotTopic(topic_data) => Some(topic_data),
            _ => None
        }
    }
}