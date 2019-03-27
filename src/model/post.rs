use actix::Message;
use chrono::NaiveDateTime;

use crate::schema::posts;
use crate::model::common::*;

use crate::model::user::SlimUser;
use crate::model::errors::ServiceError;

#[derive(Debug, Queryable, Serialize)]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub topic_id: i32,
    pub post_id: Option<i32>,
    pub post_content: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub last_reply_time: NaiveDateTime,
    pub reply_count: i32,
    pub is_locked: bool,
}

#[derive(Insertable)]
#[table_name = "posts"]
pub struct NewPost {
    pub user_id: i32,
    pub topic_id: i32,
    pub post_id: Option<i32>,
    pub post_content: String,
}

#[derive(Debug, Deserialize)]
pub struct PostRequest {
    pub post_id: Option<i32>,
    pub topic_id: i32,
    pub post_content: String,
}

#[derive(Serialize, Debug)]
pub struct PostWithSlimUser {
    pub id: i32,
    pub user: Option<SlimUser>,
    pub topic_id: i32,
    pub post_id: Option<i32>,
    pub post_content: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub last_reply_time: NaiveDateTime,
    pub reply_count: i32,
    pub is_locked: bool,
}

impl MatchUser for Post {
    fn get_user_id(&self) -> &i32 {
        &self.user_id
    }
}

impl Post {
    pub fn attach_user(self, users: &Vec<SlimUser>) -> PostWithSlimUser {
        PostWithSlimUser {
            id: self.id,
            user: self.make_user_field(users),
            topic_id: self.topic_id,
            post_id: self.post_id,
            post_content: self.post_content,
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_reply_time: self.last_reply_time,
            reply_count: self.reply_count,
            is_locked: self.is_locked,
        }
    }
}

impl Message for PostQuery {
    type Result = Result<PostQueryResult, ServiceError>;
}

pub enum PostQuery {
    AddPost(NewPost),
    EditPost(NewPost),
    GetPost(i32),
}

pub enum PostQueryResult {
    AddedPost,
    GotPost(Post),
}

impl PostQueryResult {
    pub fn to_post_data(self) -> Option<Post> {
        match self {
            PostQueryResult::GotPost(post_data) => Some(post_data),
            _ => None
        }
    }
}