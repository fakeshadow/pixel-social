use actix_web::HttpResponse;
use chrono::NaiveDateTime;

use crate::model::{
    errors::ServiceError,
    user::{User, UserRef, ToUserRef},
    common::{AttachUser, GetUserId, ResponseMessage},
};
use crate::schema::posts;
use crate::model::common::GetSelfId;

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

#[derive(Debug, Deserialize)]
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
    pub fn attach_user_id(mut self, id: Option<u32>) -> Self {
        self.user_id = id;
        self
    }
    pub fn extract_self_id(&self) -> Result<&u32, ServiceError> {
        Ok(self.id.as_ref().ok_or(ServiceError::BadRequestGeneral)?)
    }

    pub fn extract_topic_id(&self) -> Result<&u32, ServiceError> {
        Ok(self.topic_id.as_ref().ok_or(ServiceError::BadRequestGeneral)?)
    }

    pub fn make_post<'a>(&'a self, id: &'a u32) -> Result<NewPost<'a>, ServiceError> {
        Ok(NewPost {
            id,
            user_id: self.user_id.as_ref().ok_or(ServiceError::BadRequestGeneral)?,
            topic_id: self.extract_topic_id()?,
            post_id: self.post_id.as_ref(),
            post_content: self.post_content.as_ref().ok_or(ServiceError::BadRequestGeneral)?,
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
    pub post: Post,
    pub user: Option<UserRef<'a>>,
}

impl<'u> AttachUser<'u, User> for Post {
    type Output = PostWithUser<'u>;
    fn self_user_id(&self) -> &u32 { &self.user_id }
    fn attach_user(self, users: &'u Vec<User>) -> Self::Output {
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
    fn get_user_id(&self) -> &u32 { &self.user_id }
}

pub enum PostQuery<'a> {
    AddPost(&'a mut PostRequest),
    UpdatePost(&'a PostRequest),
    GetPost(&'a u32),
}

pub enum PostQueryResult<'a> {
    AddedPost,
    GotPost(&'a PostWithUser<'a>),
}

impl<'a> PostQueryResult<'a> {
    pub fn to_response(&self) -> HttpResponse {
        match self {
            PostQueryResult::AddedPost => HttpResponse::Ok().json(ResponseMessage::new("Add Post Success")),
            PostQueryResult::GotPost(post) => HttpResponse::Ok().json(&post),
        }
    }
}