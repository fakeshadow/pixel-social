use actix::Message;
use chrono::NaiveDateTime;
use crate::schema::posts;

use crate::model::errors::ServiceError;

#[derive(Identifiable, Queryable, Serialize)]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub to_pid: i32,
    pub to_tid: i32,
    pub post_content: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[table_name = "posts"]
pub struct NewPost {
    pub user_id: i32,
    pub to_pid: i32,
    pub to_tid: i32,
    pub post_content: String,
}

#[derive(Debug, Deserialize)]
pub struct PostRequest {
    pub to_pid: Option<i32>,
    pub to_tid: i32,
    pub post_content: String,
}

impl Message for PostQuery {
    type Result = Result<PostQueryResult, ServiceError>;
}

pub enum PostQuery {
    AddPost(NewPost),
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