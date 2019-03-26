use actix::Message;
use chrono::NaiveDateTime;

use crate::schema::posts;
use crate::model::topic::Topic;
use crate::model::user::SlimUser;
use crate::model::errors::ServiceError;

#[derive(Debug, Identifiable, Queryable, Serialize, Associations)]
#[belongs_to(Topic)]
#[table_name = "posts"]
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

#[derive(Debug, Serialize)]
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

impl Post {
    pub fn attach_slim_user(self, users: &Vec<SlimUser>) -> PostWithSlimUser {
        let mut _index: Vec<usize> = Vec::with_capacity(1);
        for (index, user) in users.iter().enumerate() {
            if &self.user_id == &user.id {
                _index.push(index);
                break;
            }
        };
        if _index.len() == 0 {
            return PostWithSlimUser {
                id: self.id,
                user: None,
                topic_id: self.topic_id,
                post_id: self.post_id,
                post_content: self.post_content,
                created_at: self.created_at,
                updated_at: self.updated_at,
                last_reply_time: self.last_reply_time,
                reply_count: self.reply_count,
                is_locked: self.is_locked,
            };
        }
        PostWithSlimUser {
            id: self.id,
            user: Some(users[_index[0]].clone()),
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