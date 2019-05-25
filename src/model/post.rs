use chrono::NaiveDateTime;

use crate::model::{
    common::{AttachUser, GetSelfId, GetUserId},
    errors::ServiceError,
    user::{ToUserRef, User, UserRef},
};
use crate::model::admin::AdminPrivilegeCheck;
use crate::schema::posts;

#[derive(Debug, Queryable, Serialize, Deserialize)]
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

#[derive(Insertable)]
#[table_name = "posts"]
pub struct NewPost<'a> {
    pub id: &'a u32,
    pub user_id: &'a u32,
    pub topic_id: &'a u32,
    pub post_id: Option<&'a u32>,
    pub post_content: &'a str,
    pub created_at: &'a NaiveDateTime,
    pub updated_at: &'a NaiveDateTime,
    pub last_reply_time: &'a NaiveDateTime,
}

#[derive(AsChangeset)]
#[table_name = "posts"]
pub struct UpdatePost<'a> {
    pub id: &'a u32,
    pub user_id: Option<&'a u32>,
    pub topic_id: Option<&'a u32>,
    pub post_id: Option<&'a u32>,
    pub post_content: Option<&'a str>,
    pub is_locked: Option<&'a bool>,
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
    /// pass user_id from jwt token as option for regular user updating post. pass none for admin user
    pub fn attach_user_id(&mut self, id: Option<u32>) -> &Self {
        self.user_id = id;
        self
    }
    pub fn attach_user_id_into(mut self, id: Option<u32>) -> Self {
        self.user_id = id;
        self
    }

    pub fn to_privilege_check<'a>(&'a self, level: &'a u32) -> AdminPrivilegeCheck<'a> {
        AdminPrivilegeCheck::UpdatePostCheck(level, self)
    }

    pub fn into_add_query(self) -> PostQuery { PostQuery::AddPost(self) }
    pub fn into_update_query(self) -> PostQuery { PostQuery::UpdatePost(self) }

    pub fn extract_self_id(&self) -> Result<&u32, ServiceError> {
        Ok(self.id.as_ref().ok_or(ServiceError::BadRequest)?)
    }

    pub fn extract_topic_id(&self) -> Result<&u32, ServiceError> {
        Ok(self.topic_id.as_ref().ok_or(ServiceError::BadRequest)?)
    }

    pub fn make_post<'a>(&'a self, id: &'a u32, time: &'a NaiveDateTime) -> Result<NewPost<'a>, ServiceError> {
        Ok(NewPost {
            id,
            user_id: self.user_id.as_ref().ok_or(ServiceError::BadRequest)?,
            topic_id: self.extract_topic_id()?,
            post_id: self.post_id.as_ref(),
            post_content: self.post_content.as_ref().ok_or(ServiceError::BadRequest)?,
            created_at: time,
            updated_at: time,
            last_reply_time: time,
        })
    }

    pub fn make_update(&self) -> Result<UpdatePost, ServiceError> {
        match self.user_id {
            Some(_id) => Ok(UpdatePost {
                id: self.extract_self_id()?,
                user_id: self.user_id.as_ref(),
                topic_id: None,
                post_id: None,
                post_content: self.post_content.as_ref().map(String::as_str),
                is_locked: None,
            }),
            None => Ok(UpdatePost {
                id: self.extract_self_id()?,
                user_id: None,
                topic_id: self.topic_id.as_ref(),
                post_id: self.post_id.as_ref(),
                post_content: self.post_content.as_ref().map(String::as_str),
                is_locked: self.is_locked.as_ref(),
            })
        }
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

impl GetUserId for Post {
    fn get_user_id(&self) -> u32 { self.user_id }
}

pub enum PostQuery {
    AddPost(PostRequest),
    UpdatePost(PostRequest),
    GetPost(u32),
}

pub trait IdToQuery {
    fn to_query(&self) -> PostQuery;
}

impl IdToQuery for u32 {
    fn to_query(&self) -> PostQuery {
        PostQuery::GetPost(*self)
    }
}