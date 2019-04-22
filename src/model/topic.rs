use crate::schema::topics;
use chrono::NaiveDateTime;

use crate::model::{
    user::SlimUser,
    post::PostWithUser,
    common::{GetSelfId, GetSelfTimeStamp, MatchUser, SelfHaveField}
};

#[derive(Debug, Identifiable, Queryable, Serialize, Deserialize, Clone)]
#[table_name = "topics"]
pub struct Topic {
    pub id: u32,
    pub user_id: u32,
    pub category_id: u32,
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
pub struct NewTopic<'a> {
    pub id: u32,
    pub user_id: &'a u32,
    pub category_id: &'a u32,
    pub thumbnail: &'a str,
    pub title: &'a str,
    pub body: &'a str,
}

pub struct NewTopicRequest<'a> {
    pub user_id: &'a u32,
    pub category_id: &'a u32,
    pub thumbnail: &'a str,
    pub title: &'a str,
    pub body: &'a str,
}

#[derive(Deserialize)]
pub struct TopicJson {
    pub category_id: u32,
    pub thumbnail: String,
    pub title: String,
    pub body: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TopicWithUser<T> {
    #[serde(flatten)]
    pub topic: Topic,
    pub user: Option<T>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopicWithPost {
    pub topic: Option<TopicWithUser<SlimUser>>,
    pub posts: Option<Vec<PostWithUser>>,
}

impl MatchUser for Topic {
    fn get_user_id(&self) -> &u32 {
        &self.user_id
    }
}

impl<T> GetSelfId for TopicWithUser<T> {
    fn get_self_id(&self) -> &u32 {
        &self.topic.id
    }
    fn get_self_id_copy(&self) -> u32 { self.topic.id }
}

impl TopicWithPost {
    pub fn get_topic_id(&self) -> Option<&u32> {
        match &self.posts {
            Some(posts) => Some(&posts[0].post.topic_id),
            None => None
        }
    }
    pub fn get_category_id(&self) -> Option<&u32> {
        match &self.topic {
            Some(topic) => Some(&topic.topic.category_id),
            None => None
        }
    }
}

impl SelfHaveField for TopicWithPost {
    fn have_topic(&self) -> bool {
        match &self.topic {
            Some(_topic) => true,
            None => false
        }
    }
    fn have_post(&self) -> bool {
        match &self.posts {
            Some(posts) => if !posts.is_empty() { true } else { false },
            None => false
        }
    }
}

impl<T> TopicWithUser<T>
    where T: GetSelfId {
    pub fn check_user_id(&self) -> Option<u32> {
        match &self.user {
            Some(user) => Some(user.get_self_id_copy()),
            None => None
        }
    }
}

//impl<T> GetSelfTimeStamp for TopicWithUser<T> {
//    fn get_last_reply_time(&self) -> &NaiveDateTime { &self.topic.last_reply_time }
//}

impl Topic {
    pub fn new(id: u32, request: NewTopicRequest) -> NewTopic {
        NewTopic {
            id,
            user_id: request.user_id,
            category_id: request.category_id,
            thumbnail: request.thumbnail,
            title: request.title,
            body: request.body,
        }
    }
    pub fn attach_user<T>(self, users: &Vec<T>) -> TopicWithUser<T>
        where
            T: Clone + GetSelfId,
    {
        TopicWithUser {
            user: self.make_user_field(users),
            topic: self,
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct TopicUpdateJson {
    pub id: u32,
    pub user_id: Option<u32>,
    pub category_id: Option<u32>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub thumbnail: Option<String>,
    pub is_locked: Option<bool>,
}

#[derive(AsChangeset)]
#[table_name="topics"]
pub struct TopicUpdateRequest<'a> {
    pub id: &'a u32,
    pub user_id: Option<&'a u32>,
    pub category_id: Option<&'a u32>,
    pub title: Option<&'a str>,
    pub body: Option<&'a str>,
    pub thumbnail: Option<&'a str>,
    pub is_locked: Option<&'a bool>,
}

impl<'a> TopicUpdateJson {
    pub fn get_request(&'a self, user_id: Option<&'a u32>) -> TopicUpdateRequest<'a> {
        TopicUpdateRequest {
            id: &self.id,
            user_id,
            category_id: self.category_id.as_ref(),
            title:self.title.as_ref().map(String::as_str),
            body: self.body.as_ref().map(String::as_str),
            thumbnail: self.thumbnail.as_ref().map(String::as_str),
            is_locked: self.is_locked.as_ref(),
        }
    }
}

pub enum TopicQuery<'a> {
    AddTopic(NewTopicRequest<'a>),
    GetTopic(&'a u32, &'a i64),
    UpdateTopic(TopicUpdateRequest<'a>),
}

pub enum TopicQueryResult {
    AddedTopic,
    GotTopicSlim(TopicWithPost),
}
