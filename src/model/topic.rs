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

impl Message for TopicQuery {
    type Result = Result<TopicQueryResult, ServiceError>;
}

pub enum TopicQuery {
    AddTopic(NewTopic),
    GetTopic(i32),
}

pub enum TopicQueryResult {
    AddedTopic,
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