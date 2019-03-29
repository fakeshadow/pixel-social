use actix::Message;
use chrono::NaiveDateTime;

use crate::schema::users;
use crate::model::common::{GetSelfId, Validator};
use crate::model::errors::ServiceError;

#[derive(Queryable, Insertable, Serialize)]
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

#[derive(Queryable, Serialize, Deserialize, Clone, Debug)]
pub struct SlimUser {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub avatar_url: String,
    pub signature: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable, Serialize, Deserialize, Clone, Debug)]
pub struct SlimmerUser {
    pub id: i32,
    pub username: String,
    pub avatar_url: String,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub hashed_password: &'a str,
    pub avatar_url: String,
    pub signature: String,
}

#[derive(Deserialize)]
pub struct AuthRequest {
    pub username: Option<String>,
    pub password: Option<String>,
    pub email: Option<String>,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_data: SlimUser,
}

impl Validator for AuthRequest {
    fn get_username(&self) -> &str {
        match &self.username {
            Some(username) => username,
            None => ""
        }
    }
    fn get_password(&self) -> &str {
        match &self.password {
            Some(password) => password,
            None => ""
        }
    }
    fn get_email(&self) -> &str {
        match &self.email {
            Some(email) => email,
            None => ""
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct UserUpdateRequest {
    pub id: Option<i32>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub signature: Option<String>,
    pub is_admin: Option<i32>,
    pub blocked: Option<bool>,
}

impl Validator for UserUpdateRequest {
    fn get_username(&self) -> &str {
        match &self.username {
            Some(username) => username,
            None => ""
        }
    }
    fn get_password(&self) -> &str {
        ""
    }
    fn get_email(&self) -> &str {
        ""
    }
}

impl UserUpdateRequest {
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
        if let Some(new_is_admin) = self.is_admin {
            user.is_admin = new_is_admin
        };
        if let Some(new_blocked) = self.blocked {
            user.blocked = new_blocked
        };
        Ok(user)
    }
}

impl<'a> User {
    pub fn new(username: &'a str, email: &'a str, hashed_password: &'a str) -> NewUser<'a> {
        NewUser {
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


impl GetSelfId for SlimUser {
    fn get_self_id(&self) -> &i32 {
        &self.id
    }
}

impl GetSelfId for SlimmerUser {
    fn get_self_id(&self) -> &i32 {
        &self.id
    }
}

pub enum UserQuery {
    Register(AuthRequest),
    Login(AuthRequest),
    GetMe(i32),
    GetUser(String),
    UpdateUser(UserUpdateRequest),
}

impl Message for UserQuery {
    type Result = Result<UserQueryResult, ServiceError>;
}

pub enum UserQueryResult {
    Registered,
    LoggedIn(AuthResponse),
    GotUser(User),
    GotSlimUser(SlimUser),
}