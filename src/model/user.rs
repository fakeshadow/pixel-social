use actix_web::HttpResponse;
use chrono::NaiveDateTime;

use crate::model::common::{GetSelfId, Validator, ResponseMessage};
use crate::schema::users;

#[derive(Queryable, Deserialize, Serialize, Debug)]
pub struct User {
    pub id: u32,
    pub username: String,
    pub email: String,
    pub hashed_password: String,
    pub avatar_url: String,
    pub signature: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub is_admin: u32,
    pub blocked: bool,
}

#[derive(Queryable, Deserialize, Serialize, Debug, Clone)]
pub struct SlimUser {
    pub id: u32,
    pub username: String,
    pub email: String,
    pub avatar_url: String,
    pub signature: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl<'a> User {
    pub fn slim(self) -> SlimUser {
        SlimUser {
            id: self.id,
            username: self.username,
            email: self.email,
            avatar_url: self.avatar_url,
            signature: self.signature,
            created_at: self.created_at,
            updated_at: self.updated_at,
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
    pub user_data: SlimUser,
}

impl Validator for AuthJson {
    fn get_username(&self) -> &str {
        &self.username
    }
    fn get_password(&self) -> &str {
        &self.password
    }
    fn get_email(&self) -> &str {
        match &self.email {
            Some(email) => email,
            None => "",
        }
    }
}

#[derive(Deserialize)]
pub struct UserUpdateJson {
    pub id: Option<u32>,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub signature: Option<String>,
    pub is_admin: Option<u32>,
    pub blocked: Option<bool>,
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
        }
    }
}

impl Validator for UserUpdateJson {
    fn get_username(&self) -> &str {
        match &self.username {
            Some(username) => username,
            None => "",
        }
    }
    fn get_password(&self) -> &str {
        ""
    }
    fn get_email(&self) -> &str {
        ""
    }
}

impl GetSelfId for SlimUser {
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

pub enum UserQueryResult {
    Registered,
    LoggedIn(AuthResponse),
    GotUser(User),
    GotSlimUser(SlimUser),
}

impl UserQueryResult {
    pub fn to_response(&self) -> HttpResponse {
        match self {
            UserQueryResult::GotSlimUser(slim_user) => HttpResponse::Ok().json(&slim_user),
            UserQueryResult::GotUser(user) => HttpResponse::Ok().json(&user),
            UserQueryResult::LoggedIn(login_data) => HttpResponse::Ok().json(&login_data),
            UserQueryResult::Registered => HttpResponse::Ok().json(ResponseMessage::new("Register Success"))
        }
    }
}