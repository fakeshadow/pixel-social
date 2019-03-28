use actix::Message;
use chrono::NaiveDateTime;

use crate::schema::topics;

use crate::model::common::{GetSelfId, MatchUser, GetSelfTimeStamp};
use crate::model::post::PostWithSlimUser;
use crate::model::user::{SlimUser, SlimmerUser};
use crate::model::errors::ServiceError;

#[derive(Debug, Identifiable, Queryable, Serialize, Clone)]
#[table_name = "topics"]
pub struct Topic {
    pub id: i32,
    #[serde(skip_serializing)]
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

#[derive(Serialize, Debug, Clone)]
pub struct TopicWithUser<T> {
    #[serde(flatten)]
    pub topic: Topic,
    pub user: Option<T>,
}

#[derive(Debug, Serialize)]
pub struct TopicResponseSlim {
    pub topic: Option<TopicWithUser<SlimUser>>,
    pub posts: Option<Vec<PostWithSlimUser>>,
}

impl MatchUser for Topic {
    fn get_user_id(&self) -> &i32 { &self.user_id }
}

impl<T> GetSelfTimeStamp for TopicWithUser<T> {
    fn get_last_reply_time(&self) -> &NaiveDateTime { &self.topic.last_reply_time }
}

impl Topic {
    pub fn attach_user<T>(self, users: &Vec<T>) -> TopicWithUser<T>
        where T: Clone + GetSelfId {
        TopicWithUser {
            user: self.make_user_field(users),
            topic: self,
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
    GotTopicSlim(TopicResponseSlim),
}
