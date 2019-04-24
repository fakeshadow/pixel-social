use actix_web::HttpResponse;
use chrono::NaiveDateTime;

use crate::model::{
    errors::ServiceError,
    user::SlimUser,
    common::{MatchUser, GetSelfId, ResponseMessage},
};
use crate::schema::posts;
use crate::model::common::CheckUserId;

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
pub struct PostJson {
    pub id: Option<u32>,
    pub user_id: Option<u32>,
    pub topic_id: Option<u32>,
    pub post_id: Option<u32>,
    pub post_content: Option<String>,
    pub is_locked: Option<bool>,
}

impl<'a> PostJson {
    /// pass user_id from jwt token as option for regular user updating post. pass none for admin user
    pub fn to_request(&'a self, id: Option<&'a u32>) -> PostRequest<'a> {
        PostRequest {
            id: self.id.as_ref(),
            user_id: id,
            topic_id: self.topic_id.as_ref(),
            post_id: self.post_id.as_ref(),
            post_content: self.post_content.as_ref().map(String::as_str),
            is_locked: self.is_locked.as_ref(),
        }
    }
}

pub struct PostRequest<'a> {
    pub id: Option<&'a u32>,
    pub user_id: Option<&'a u32>,
    pub topic_id: Option<&'a u32>,
    pub post_id: Option<&'a u32>,
    pub post_content: Option<&'a str>,
    pub is_locked: Option<&'a bool>,
}

impl<'a> PostRequest<'a> {
    pub fn extract_self_id(&self) -> Result<&'a u32, ServiceError> {
        Ok(self.id.ok_or(ServiceError::BadRequestGeneral)?)
    }

    pub fn extract_topic_id(&self) -> Result<&'a u32, ServiceError> {
        Ok(self.topic_id.ok_or(ServiceError::BadRequestGeneral)?)
    }

    pub fn make_post(&self, id: &'a u32) -> Result<NewPost<'a>, ServiceError> {
        Ok(NewPost {
            id,
            user_id: self.user_id.ok_or(ServiceError::BadRequestGeneral)?,
            topic_id: self.extract_topic_id()?,
            post_id: self.post_id,
            post_content: self.post_content.ok_or(ServiceError::BadRequestGeneral)?,
        })
    }

    pub fn make_update(&self) -> Result<UpdatePost, ServiceError> {
        match self.user_id {
            Some(id) => Ok(UpdatePost {
                id: self.extract_self_id()?,
                user_id: self.user_id,
                topic_id: None,
                post_id: None,
                post_content: self.post_content,
                is_locked: None,
            }),
            None => Ok(UpdatePost {
                id: self.extract_self_id()?,
                user_id: None,
                topic_id: self.topic_id,
                post_id: self.post_id,
                post_content: self.post_content,
                is_locked: self.is_locked,
            })
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PostWithUser {
    #[serde(flatten)]
    pub post: Post,
    pub user: Option<SlimUser>,
}

/// extract self user and self post from post with user
impl CheckUserId<SlimUser, Post> for PostWithUser {
    fn get_self_user(&self) -> Option<&SlimUser> {
        self.user.as_ref()
    }
    fn get_self_post_topic(&self) -> &Post {
        &self.post
    }
}


impl GetSelfId for PostWithUser {
    fn get_self_id(&self) -> &u32 { &self.post.id }
    fn get_self_id_copy(&self) -> u32 { self.post.id }
}

impl MatchUser for Post {
    fn get_user_id(&self) -> &u32 {
        &self.user_id
    }
}

impl Post {
    pub fn attach_user(self, users: &Vec<SlimUser>) -> PostWithUser {
        PostWithUser {
            user: self.make_user_field(users),
            post: self,
        }
    }
}

pub enum PostQuery<'a> {
    AddPost(&'a mut PostRequest<'a>),
    UpdatePost(&'a PostRequest<'a>),
    GetPost(&'a u32),
}

pub enum PostQueryResult {
    AddedPost,
    GotPost(Post),
}

impl PostQueryResult {
    pub fn to_response(&self) -> HttpResponse {
        match self {
            PostQueryResult::AddedPost => HttpResponse::Ok().json(ResponseMessage::new("Add Post Success")),
            PostQueryResult::GotPost(post) => HttpResponse::Ok().json(&post),
        }
    }
}