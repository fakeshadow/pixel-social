use actix::{Actor, SyncContext};
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use chrono::{NaiveDateTime, Local};
use std::convert::From;

use crate::schema::users;

pub struct DbExecutor(pub Pool<ConnectionManager<PgConnection>>);

impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable)]
#[table_name = "users"]
pub struct User {
    pub uid:  u32,
    pub username:  String,
    pub email:  String,
    pub password:  String,
    pub avatar_url:  String,
    pub signature:  String,
    pub created_at:  NaiveDateTime,
}

impl User {
    pub fn create(uid: u32, username: String, email: String, password: String) -> Self {
        User {
            uid,
            username,
            email,
            password,
            // change to default avatar url later
            avatar_url: String::from(""),
            signature: String::from(""),
            created_at: Local::now().naive_local(),
        }
    }
}
