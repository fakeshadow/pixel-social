use actix_web::HttpResponse;
use chrono::NaiveDateTime;

use crate::model::{
    errors::ServiceError,
    common::{GetSelfId, Validator, ResponseMessage}
};
use crate::schema::users;
use std::iter::FromIterator;

#[derive(Queryable, Deserialize, Serialize, Clone, Debug)]
pub struct User {
    pub id: u32,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub hashed_password: String,
    pub avatar_url: String,
    pub signature: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub is_admin: u32,
    pub blocked: bool,
    pub show_email: bool,
    pub show_created_at: bool,
    pub show_updated_at: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PublicUser {
    pub id: u32,
    pub username: String,
    pub email: Option<String>,
    pub avatar_url: String,
    pub signature: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub is_admin: u32,
    pub blocked: bool,
    pub show_email: bool,
    pub show_created_at: bool,
    pub show_updated_at: bool,
}

impl Into<PublicUser> for User {
    fn into(self) -> PublicUser {
        let email = if self.show_email { Some(self.email) } else { None };
        let created_at = if self.show_created_at { Some(self.created_at) } else { None };
        let updated_at = if self.show_updated_at { Some(self.updated_at) } else { None };
        PublicUser {
            id: self.id,
            username: self.username,
            email,
            avatar_url: self.avatar_url,
            signature: self.signature,
            created_at,
            updated_at,
            is_admin: self.is_admin,
            blocked: self.blocked,
            show_email: self.show_email,
            show_created_at: self.show_created_at,
            show_updated_at: self.show_updated_at,
        }
    }
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub id: &'a u32,
    pub username: &'a str,
    pub email: &'a str,
    pub hashed_password: &'a str,
    pub avatar_url: &'a str,
    pub signature: &'a str,
}

#[derive(Deserialize)]
pub struct AuthJson {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
}

impl AuthJson {
    pub fn to_request(&self) -> AuthRequest {
        AuthRequest {
            username: &self.username,
            password: &self.password,
            email: self.email.as_ref().map(String::as_str),
        }
    }
}

pub struct AuthRequest<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub email: Option<&'a str>,
}

impl<'a> AuthRequest<'a> {
    pub fn make_user(&self, id: &'a u32, hashed_password: &'a str) -> NewUser<'a> {
        NewUser {
            id,
            username: self.username,
            // ToDo: In case validator failed and cause unwrap panic.
            email: self.email.unwrap(),
            hashed_password,
            avatar_url: "",
            signature: "",
        }
    }
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_data: PublicUser,
}

#[derive(Deserialize)]
pub struct UserUpdateJson {
    pub id: Option<u32>,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub signature: Option<String>,
    pub is_admin: Option<u32>,
    pub blocked: Option<bool>,
    pub show_email: Option<bool>,
    pub show_created_at: Option<bool>,
    pub show_updated_at: Option<bool>,
}

#[derive(AsChangeset)]
#[table_name = "users"]
pub struct UserUpdateRequest<'a> {
    pub id: &'a u32,
    pub username: Option<&'a str>,
    pub avatar_url: Option<&'a str>,
    pub signature: Option<&'a str>,
    pub is_admin: Option<&'a u32>,
    pub blocked: Option<&'a bool>,
    pub show_email: Option<&'a bool>,
    pub show_created_at: Option<&'a bool>,
    pub show_updated_at: Option<&'a bool>,
}

impl<'a> UserUpdateJson {
    pub fn to_request(&'a self, id: &'a u32) -> UserUpdateRequest<'a> {
        UserUpdateRequest {
            id,
            username: self.username.as_ref().map(String::as_str),
            avatar_url: self.avatar_url.as_ref().map(String::as_str),
            signature: self.signature.as_ref().map(String::as_str),
            is_admin: None,
            blocked: None,
            show_email: self.show_email.as_ref(),
            show_created_at: self.show_created_at.as_ref(),
            show_updated_at: self.show_updated_at.as_ref(),
        }
    }
    pub fn to_request_admin(&'a self, id: &'a u32) -> UserUpdateRequest<'a> {
        UserUpdateRequest {
            id,
            username: None,
            avatar_url: None,
            signature: None,
            is_admin: self.is_admin.as_ref(),
            blocked: self.blocked.as_ref(),
            show_email: None,
            show_created_at: None,
            show_updated_at: None,
        }
    }
}

impl GetSelfId for User {
    fn get_self_id(&self) -> &u32 {
        &self.id
    }
    fn get_self_id_copy(&self) -> u32 {
        self.id
    }
}

impl GetSelfId for PublicUser {
    fn get_self_id(&self) -> &u32 {
        &self.id
    }
    fn get_self_id_copy(&self) -> u32 {
        self.id
    }
}

/// impl for query enum is in handler
pub enum UserQuery<'a> {
    Register(&'a AuthRequest<'a>),
    Login(&'a AuthRequest<'a>),
    GetMe(&'a u32),
    GetUser(&'a str),
    UpdateUser(&'a UserUpdateRequest<'a>),
}

impl<'a> Validator for UserQuery<'a> {
    // ToDo: handle update validation separately.
    fn get_username(&self) -> &str {
        match self {
            UserQuery::Login(req) => req.username,
            UserQuery::GetUser(username) => username,
            UserQuery::Register(req) => req.username,
            UserQuery::UpdateUser(req) => req.username.unwrap_or(""),
            _ => ""
        }
    }
    fn get_password(&self) -> &str {
        match self {
            UserQuery::Register(req) => req.password,
            _ => ""
        }
    }
    fn get_email(&self) -> &str {
        match self {
            UserQuery::Register(req) => req.email.unwrap_or(""),
            _ => ""
        }
    }
}

pub enum UserQueryResult {
    Registered,
    LoggedIn(AuthResponse),
    GotUser(User),
    GotPublicUser(PublicUser),
}

impl UserQueryResult {
    pub fn to_response(&self) -> HttpResponse {
        match self {
            UserQueryResult::GotPublicUser(public_user) =>HttpResponse::Ok().json(&public_user),
            UserQueryResult::GotUser(user) => HttpResponse::Ok().json(&user),
            UserQueryResult::LoggedIn(login_data) => HttpResponse::Ok().json(&login_data),
            UserQueryResult::Registered => HttpResponse::Ok().json(ResponseMessage::new("Register Success"))
        }
    }
}