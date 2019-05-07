use chrono::NaiveDateTime;

use crate::model::{admin::AdminPrivilegeCheck, common::{GetSelfId, Validator}, errors::ServiceError};
use crate::model::mail::Mail;
use crate::schema::users;
use crate::handler::user_async::UserQueryAsync;

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

/// user ref is attached to post and topic after privacy filter.
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

impl User {
    pub fn to_mail(&self) -> Mail {
        Mail::from_user(self)
    }
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
    pub fn to_register_query(&self) -> UserQuery {
        UserQuery::Register(self)
    }
    pub fn to_login_query(&self) -> UserQuery {
        UserQuery::Login(self)
    }

    /// for async query
    pub fn into_register_query(self) -> UserQueryAsync { UserQueryAsync::Register(self) }
    pub fn into_login_query(self) -> UserQueryAsync {
        UserQueryAsync::Login(self)
    }

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
    pub user_data: UserRef<'a>,
}

#[derive(Serialize)]
pub struct AuthResponseAsync {
    pub token: String,
    pub user: User,
}

impl AuthResponseAsync {
    pub fn to_response(&self) -> AuthResponse {
        AuthResponse {
            token: &self.token,
            user_data: self.user.to_ref(),
        }
    }
}

#[derive(Deserialize)]
pub struct UserUpdateRequest {
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
pub struct UpdateUser<'a> {
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

impl UserUpdateRequest {
    /// pass user_id from jwt for normal update user. pass none for admin update user.
    pub fn attach_id(&mut self, id: Option<u32>) -> &Self {
        match id {
            Some(_) => {
                self.id = id;
                self.is_admin = None;
                self.blocked = None;
                self
            }
            None => {
                self.username = None;
                self.avatar_url = None;
                self.signature = None;
                self.show_email = None;
                self.show_created_at = None;
                self.show_updated_at = None;
                self
            }
        }
    }
    pub fn attach_id_async(mut self, id: Option<u32>) -> Self {
        match id {
            Some(_) => {
                self.id = id;
                self.is_admin = None;
                self.blocked = None;
                self
            }
            None => {
                self.username = None;
                self.avatar_url = None;
                self.signature = None;
                self.show_email = None;
                self.show_created_at = None;
                self.show_updated_at = None;
                self
            }
        }
    }

    pub fn extract_id(&self) -> Result<&u32, ServiceError> {
        self.id.as_ref().ok_or(ServiceError::BadRequestGeneral)
    }

    pub fn make_update(&self) -> Result<UpdateUser, ServiceError> {
        Ok(UpdateUser {
            id: self.extract_id()?,
            username: self.username.as_ref().map(String::as_str),
            avatar_url: self.avatar_url.as_ref().map(String::as_str),
            signature: self.signature.as_ref().map(String::as_str),
            is_admin: self.is_admin.as_ref(),
            blocked: self.blocked.as_ref(),
            show_email: self.show_email.as_ref(),
            show_created_at: self.show_created_at.as_ref(),
            show_updated_at: self.show_updated_at.as_ref(),
        })
    }
}

impl UserUpdateRequest {
    pub fn to_privilege_check<'a>(&'a self, level: &'a u32) -> AdminPrivilegeCheck<'a> {
        AdminPrivilegeCheck::UpdateUserCheck(level, self)
    }
    pub fn into_update_query(self) -> UserQueryAsync { UserQueryAsync::UpdateUser(self) }

    pub fn to_query(&self) -> UserQuery { UserQuery::UpdateUser(self) }
}

pub enum UserQuery<'a> {
    Register(&'a AuthRequest),
    Login(&'a AuthRequest),
    GetMe(u32),
    GetUser(u32),
    UpdateUser(&'a UserUpdateRequest),
}

/// method is into_query when consume self. to_query when only ref self
pub trait IdToQuery {
    fn into_query<'a>(self, jwt_id: u32) -> UserQuery<'a>;
}

impl IdToQuery for u32 {
    fn into_query<'a>(self, jwt_id: u32) -> UserQuery<'a> {
        if jwt_id == self {
            UserQuery::GetMe(jwt_id)
        } else {
            UserQuery::GetUser(self)
        }
    }
}

pub trait IdToQueryAsync {
    fn into_query(self) -> UserQueryAsync;
}

impl IdToQueryAsync for u32 {
    fn into_query(self) -> UserQueryAsync {
        UserQueryAsync::GetUser(self)
    }
}


impl<'a> Validator for UserQuery<'a> {
    // ToDo: handle update validation separately.
    fn get_username(&self) -> &str {
        match self {
            UserQuery::Login(req) => &req.username,
            UserQuery::Register(req) => &req.username,
            UserQuery::UpdateUser(req) => req.username.as_ref().map(String::as_str).unwrap_or(""),
            _ => ""
        }
    }
    fn get_password(&self) -> &str {
        match self {
            UserQuery::Register(req) => &req.password,
            UserQuery::Login(req) => &req.password,
            _ => ""
        }
    }
    fn get_email(&self) -> &str {
        match self {
            UserQuery::Register(req) => req.email.as_ref().map(String::as_str).unwrap_or(""),
            _ => ""
        }
    }
}