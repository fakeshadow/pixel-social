use chrono::NaiveDateTime;

use crate::model::{
    common::{AttachUser, GetSelfId},
    errors::ServiceError,
    user::{ToUserRef, User, UserRef},
};

#[derive(Serialize, Deserialize)]
pub struct Post {
    pub id: u32,
    pub user_id: u32,
    pub topic_id: u32,
    pub post_id: Option<u32>,
    pub post_content: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub last_reply_time: NaiveDateTime,
    pub reply_count: i32,
    pub is_locked: bool,
}

#[derive(Deserialize)]
pub struct PostRequest {
    pub id: Option<u32>,
    pub user_id: Option<u32>,
    pub topic_id: Option<u32>,
    pub post_id: Option<u32>,
    pub post_content: Option<String>,
    pub is_locked: Option<bool>,
}

impl PostRequest {
    pub fn attach_user_id(mut self, id: Option<u32>) -> Self {
        self.user_id = id;
        self
    }

    pub fn make_new(self) -> Result<Self, ServiceError> {
        if self.topic_id.is_none() ||
            self.post_content.is_none() {
            Err(ServiceError::BadRequest)
        } else {
            Ok(self)
        }
    }

    pub fn make_update(mut self) -> Result<Self, ServiceError> {
        if self.id.is_none() {
            return Err(ServiceError::BadRequest);
        }
        if let Some(uid) = self.user_id {
            self.topic_id = None;
            self.post_id = None;
            self.is_locked = None;
        }
        Ok(self)
    }
}

#[derive(Serialize)]
pub struct PostWithUser<'a> {
    #[serde(flatten)]
    pub post: &'a Post,
    pub user: Option<UserRef<'a>>,
}

impl<'u, > AttachUser<'u, User> for Post {
    type Output = PostWithUser<'u>;
    fn self_user_id(&self) -> &u32 { &self.user_id }
    fn attach_user(&'u self, users: &'u Vec<User>) -> Self::Output {
        PostWithUser {
            user: self.make_field(&users),
            post: self,
        }
    }
}

impl GetSelfId for Post {
    fn get_self_id(&self) -> &u32 { &self.id }
}