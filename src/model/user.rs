use actix::Message;
use actix_web::Json;
use chrono::NaiveDateTime;
use crate::schema::users;

use crate::model::errors::ServiceError;

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable)]
#[table_name = "users"]
pub struct User {
    pub id: i32,
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
    pub id: i32,
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

#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    pub id: Option<i32>,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub signature: Option<String>,
}

impl UpdateRequest {
    pub fn new(raw_request: Json<UpdateRequest>, user_id: i32) -> UpdateRequest {
        UpdateRequest {
            id: Some(user_id),
            username: raw_request.username.clone(),
            avatar_url: raw_request.avatar_url.clone(),
            signature: raw_request.signature.clone(),
        }
    }

    pub fn update_user_data(self, mut user: User) -> Result<User, ()> {
        if let Some(new_username) = self.username {
            user.username = new_username
        };
        if let Some(new_avatar_url) = self.avatar_url {
            user.avatar_url = new_avatar_url
        };
        if let Some(new_signature) = self.signature {
            user.signature = new_signature
        };
        Ok(user)
    }
}

pub enum UserQuery {
    Register(RegisterRequest),
    Login(LoginRequest),
    GetMe(i32),
    GetUser(String),
    UpdateUser(UpdateRequest),
}

impl Message for UserQuery {
    type Result = Result<UserQueryResult, ServiceError>;
}

pub enum UserQueryResult {
    Registered,
    LoggedIn(LoginData),
    GotUser(User)
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

