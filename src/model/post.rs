use actix::Message;
use chrono::NaiveDateTime;
use crate::schema::posts;

use crate::model::errors::ServiceError;

#[derive(Debug, Serialize, Queryable, Insertable)]
#[table_name = "posts"]
pub struct Post {
    pub pid: i32,
    pub uid: i32,
    pub to_pid: i32,
    pub to_tid: i32,
    pub post_content: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Deserialize, Insertable)]
#[table_name = "posts"]
pub struct IncomingPost {
    pub uid: i32,
    pub to_pid: Option<i32>,
    pub to_tid: i32,
    pub post_content: String,
}

impl Message for IncomingPost {
    type Result = Result<(), ServiceError>;
}
