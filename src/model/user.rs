use actix_web::HttpResponse;
use chrono::NaiveDateTime;

use crate::model::{
    errors::ServiceError,
    common::{GetSelfId, Validator, ResponseMessage},
};
use crate::schema::users;

#[derive(Queryable, Serialize, Deserialize, Debug)]
pub struct User {
    pub id: u32,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    #[serde(default = "default_password")]
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
fn default_password() -> String {
    "1".to_string()
}

#[derive(Serialize)]
pub struct UserRef<'a> {
    pub id: &'a u32,
    pub username: &'a str,
    pub email: Option<&'a str>,
    pub avatar_url: &'a str,
    pub signature: &'a str,
    pub created_at: Option<&'a NaiveDateTime>,
    pub updated_at: Option<&'a NaiveDateTime>,
    pub is_admin: &'a u32,
    pub blocked: &'a bool,
    pub show_email: &'a bool,
    pub show_created_at: &'a bool,
    pub show_updated_at: &'a bool,
}

pub trait ToUserRef {
    fn to_ref(&self) -> UserRef;
}

impl ToUserRef for User {
    fn to_ref(&self) -> UserRef {
        let email = if self.show_email { Some(self.email.as_str()) } else { None };
        let created_at = if self.show_created_at { Some(&self.created_at) } else { None };
        let updated_at = if self.show_updated_at { Some(&self.updated_at) } else { None };
        UserRef {
            id: &self.id,
            username: self.username.as_str(),
            email,
            avatar_url: self.avatar_url.as_str(),
            signature: self.signature.as_str(),
            created_at,
            updated_at,
            is_admin: &self.is_admin,
            blocked: &self.blocked,
            show_email: &self.show_email,
            show_created_at: &self.show_created_at,
            show_updated_at: &self.show_updated_at,
        }
    }
}

impl GetSelfId for User {
    fn get_self_id(&self) -> &u32 { &self.id }
}

impl<'a> GetSelfId for UserRef<'a> {
    fn get_self_id(&self) -> &u32 { &self.id }
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
pub struct AuthRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
}

impl AuthRequest {
    pub fn extract_email(&self) -> Result<&str, ServiceError> {
        self.email.as_ref().map(String::as_str).ok_or(ServiceError::BadRequestGeneral)
    }

    pub fn make_user<'a>(&'a self, id: &'a u32, hashed_password: &'a str) -> Result<NewUser<'a>, ServiceError> {
        Ok(NewUser {
            id,
            username: &self.username,
            // ToDo: In case validator failed and cause unwrap panic.
            email: self.extract_email()?,
            hashed_password,
            avatar_url: "",
            signature: "",
        })
    }
}

#[derive(Serialize)]
pub struct AuthResponse<'a> {
    pub token: &'a str,
    pub user_data: &'a UserRef<'a>,
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

pub enum UserQuery<'a> {
    Register(&'a AuthRequest),
    Login(&'a AuthRequest),
    GetMe(&'a u32),
    GetUser(&'a str),
    UpdateUser(&'a UserUpdateRequest<'a>),
}

impl<'a> Validator for UserQuery<'a> {
    // ToDo: handle update validation separately.
    fn get_username(&self) -> &str {
        match self {
            UserQuery::Login(req) => &req.username,
            UserQuery::GetUser(username) => &username,
            UserQuery::Register(req) => &req.username,
            UserQuery::UpdateUser(req) => req.username.unwrap_or(""),
            _ => ""
        }
    }
    fn get_password(&self) -> &str {
        match self {
            UserQuery::Register(req) => &req.password,
            _ => ""
        }
    }
    fn get_email(&self) -> &str {
        match self {
            UserQuery::Register(req) => req.email.as_ref().map(String::as_str).unwrap_or(""),
            _ => ""
        }
    }
    fn validate(&self) -> Result<(), ServiceError> {
        Ok(())
    }
}

pub enum UserQueryResult<'a> {
    Registered,
    LoggedIn(&'a AuthResponse<'a>),
    GotUser(&'a User),
    GotPublicUser(&'a UserRef<'a>),
}

impl<'a> UserQueryResult<'a> {
    pub fn to_response(&self) -> HttpResponse {
        match self {
            UserQueryResult::GotPublicUser(public_user) => HttpResponse::Ok().json(&public_user),
            UserQueryResult::GotUser(user) => HttpResponse::Ok().json(&user),
            UserQueryResult::LoggedIn(login_data) => HttpResponse::Ok().json(&login_data),
            UserQueryResult::Registered => HttpResponse::Ok().json(ResponseMessage::new("Register Success"))
        }
    }
}