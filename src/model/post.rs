use chrono::NaiveDateTime;

use crate::model::{common::*, errors::ServiceError, user::SlimUser};
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
    pub id: u32,
    pub user_id: &'a u32,
    pub topic_id: &'a u32,
    pub post_id: Option<&'a u32>,
    pub post_content: &'a str,
}

pub struct PostRequest<'a> {
    pub user_id: &'a u32,
    pub topic_id: &'a u32,
    pub post_id: Option<&'a u32>,
    pub post_content: &'a str,
}

pub struct UpdatePostRequest<'a> {
    pub id: &'a u32,
    pub user_id: &'a u32,
    pub post_content: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct PostJson {
    pub post_id: Option<u32>,
    pub topic_id: u32,
    pub post_content: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePostJson {
    pub id: u32,
    pub post_content: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PostWithUser {
    #[serde(flatten)]
    pub post: Post,
    pub user: Option<SlimUser>,
}

impl PostWithUser {
    pub fn check_user_id(&self) -> Option<u32> {
        match &self.user {
            Some(user) => Some(user.get_self_id_copy()),
            None => None
        }
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
    pub fn new(id: u32, post_request: PostRequest) -> NewPost {
        NewPost {
            id,
            user_id: post_request.user_id,
            topic_id: post_request.topic_id,
            post_id: post_request.post_id,
            post_content: post_request.post_content,
        }
    }

    pub fn attach_user(self, users: &Vec<SlimUser>) -> PostWithUser {
        PostWithUser {
            user: self.make_user_field(users),
            post: self,
        }
    }
}

pub enum PostQuery<'a> {
    AddPost(PostRequest<'a>),
    EditPost(UpdatePostRequest<'a>),
    GetPost(u32),
}

pub enum PostQueryResult {
    AddedPost,
    GotPost(Post),
}
