use std::convert::From;

use actix::Message;
use chrono::NaiveDateTime;
use crate::schema::users;

use crate::model::errors::ServiceError;

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable)]
#[table_name = "users"]
pub struct User {
    pub uid: i32,
    pub username: String,
    pub email: String,
    pub hashed_password: String,
    pub avatar_url: String,
    pub signature: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub is_admin: i32,
    pub blocked: bool,
}

#[derive(Debug, Serialize)]
pub struct SlimUser {
    pub uid: i32,
    pub username: String,
    pub email: String,
    pub avatar_url: String,
    pub signature: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[table_name = "users"]
pub struct RegisterUserData<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub hashed_password: &'a str,
    pub avatar_url: String,
    pub signature: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginData {
    pub token: String,
    pub user_data: SlimUser,
}

pub enum UserQuery {
    Register(RegisterRequest),
    Login(LoginRequest),
    GetMe(i32),
    GetUser(String),
}

impl Message for UserQuery {
    type Result = Result<UserQueryResult, ServiceError>;
}

pub enum UserQueryResult {
    Registered,
    LoggedIn(LoginData),
    GotUser(User),
}

impl UserQueryResult {
    pub fn to_login_data(self) -> Option<LoginData> {
        match self {
            UserQueryResult::LoggedIn(login_data) => Some(login_data),
            _ => None
        }
    }
    pub fn to_user_data(self) -> Option<SlimUser> {
        match self {
            UserQueryResult::GotUser(user) => Some(user.slim()),
            _ => None
        }
    }
}

impl<'a> User {
    pub fn new(username: &'a str, email: &'a str, hashed_password: &'a str) -> RegisterUserData<'a> {
        RegisterUserData {
            username,
            email,
            hashed_password,
            // change to default avatar url later
            avatar_url: String::from(""),
            signature: String::from(""),
        }
    }
    pub fn slim(self) -> SlimUser {
        SlimUser {
            uid: self.uid,
            username: self.username,
            email: self.email,
            avatar_url: self.avatar_url,
            signature: self.signature,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

