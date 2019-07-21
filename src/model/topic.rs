use chrono::NaiveDateTime;

use crate::model::{
    common::{AttachUser, GetSelfId, GetUserId},
    errors::ServiceError,
    post::{Post, PostWithUser},
    user::{ToUserRef, User, UserRef},
};

#[derive(Serialize, Debug, Clone)]
// ToDo: add field for topic visiable
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
    pub is_locked: bool,
    pub reply_count: Option<u32>,
}

#[derive(Deserialize)]
pub struct TopicRequest {
    pub id: Option<u32>,
    pub user_id: Option<u32>,
    pub category_id: u32,
    pub title: Option<String>,
    pub body: Option<String>,
    pub thumbnail: Option<String>,
    pub is_locked: Option<bool>,
}

impl TopicRequest {
    pub fn attach_user_id(mut self, id: Option<u32>) -> Self {
        self.user_id = id;
        self
    }
    pub fn check_new(&self) -> Result<(), ServiceError> {
        if self.title.is_none() ||
            self.body.is_none() ||
            self.thumbnail.is_none() {
            Err(ServiceError::BadRequest)
        } else {
            Ok(())
        }
    }
    pub fn check_update(&mut self) -> Result<(), ServiceError> {
        if self.id.is_none() {
            return Err(ServiceError::BadRequest);
        }
        if let Some(_) = self.user_id {
            self.is_locked = None;
        }
        Ok(())
    }
}

impl GetSelfId for Topic {
    fn self_id(&self) -> &u32 { &self.id }
}

impl GetUserId for Topic {
    fn get_user_id(&self) -> u32 { self.user_id }
}

impl<'u, T> AttachUser<'u, T> for Topic
    where T: GetSelfId + ToUserRef {
    type Output = TopicWithUser<'u>;
    fn self_user_id(&self) -> &u32 { &self.user_id }
    fn attach_user(&'u self, users: &'u Vec<T>) -> Self::Output {
        TopicWithUser {
            user: self.make_field(&users),
            topic: self,
        }
    }
}

#[derive(Serialize)]
pub struct TopicWithUser<'a> {
    #[serde(flatten)]
    pub topic: &'a Topic,
    pub user: Option<UserRef<'a>>,
}

impl<'a> TopicWithUser<'a> {
    pub fn new(t: &'a Vec<Topic>, u: &'a Vec<User>) -> Vec<Self> {
        t.iter().map(|t| t.attach_user(&u)).collect()
    }

}

#[derive(Serialize)]
pub struct TopicWithPost<'a> {
    pub topic: Option<TopicWithUser<'a>>,
    pub posts: Vec<PostWithUser<'a>>,
}

impl<'a> TopicWithPost<'a> {
    pub fn new(t: Option<&'a Topic>, p: &'a Vec<Post>, u: &'a Vec<User>) -> Self {
        TopicWithPost {
            topic: t.map(|t| t.attach_user(&u)),
            posts: p.iter().map(|p| p.attach_user(&u)).collect(),
        }
    }
}