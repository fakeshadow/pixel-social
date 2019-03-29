use actix::Message;
use chrono::NaiveDateTime;

use crate::schema::posts;
use crate::model::common::*;

use crate::model::user::SlimUser;
use crate::model::errors::ServiceError;

#[derive(Debug, Queryable, Serialize, Deserialize)]
pub struct Post {
    pub id: i32,
    #[serde(skip_serializing, skip_deserializing)]
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

#[derive(Serialize,Deserialize, Debug)]
pub struct PostWithSlimUser {
    #[serde(flatten)]
    pub post: Post,
    pub user: Option<SlimUser>,
}

impl MatchUser for Post {
    fn get_user_id(&self) -> &i32 {
        &self.user_id
    }
}

impl Post {
    pub fn attach_user(self, users: &Vec<SlimUser>) -> PostWithSlimUser {
        PostWithSlimUser {
            user: self.make_user_field(users),
            post: self
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