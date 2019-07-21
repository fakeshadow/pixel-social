use chrono::NaiveDateTime;

use crate::model::{
    common::{AttachUser, GetSelfId, GetUserId},
    errors::ServiceError,
    user::{ User, UserRef},
};

#[derive(Serialize, Deserialize)]
pub struct Post {
    pub id: u32,
    pub user_id: u32,
    pub topic_id: u32,
    pub category_id: u32,
    pub post_id: Option<u32>,
    pub post_content: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub last_reply_time: NaiveDateTime,
    pub is_locked: bool,
    pub reply_count: Option<u32>,
}

#[derive(Deserialize)]
pub struct PostRequest {
    pub id: Option<u32>,
    pub user_id: Option<u32>,
    pub topic_id: Option<u32>,
    pub category_id: u32,
    pub post_id: Option<u32>,
    pub post_content: Option<String>,
    pub is_locked: Option<bool>,
}

impl PostRequest {
    pub fn attach_user_id(mut self, id: Option<u32>) -> Self {
        self.user_id = id;
        self
    }

    pub fn check_new(&self) -> Result<(), ServiceError> {
        if self.topic_id.is_none() ||
            self.post_content.is_none() {
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
            self.topic_id = None;
            self.post_id = None;
            self.is_locked = None;
        }
        Ok(())
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
    fn self_id(&self) -> &u32 { &self.id }
}

impl GetUserId for Post {
    fn get_user_id(&self) -> u32 { self.user_id }
}