use chrono::NaiveDateTime;

use crate::model::{
    mail::Mail,
    errors::ServiceError,
    common::{GetSelfId, Validator},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
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
    pub privilege: u32,
    pub show_email: bool,
    pub show_created_at: bool,
    pub show_updated_at: bool,
}

fn default_password() -> String {
    "1".to_string()
}

//user ref is attached to post and topic after privacy filter.
#[derive(Serialize)]
pub struct UserRef<'a> {
    pub id: &'a u32,
    pub username: &'a str,
    pub email: Option<&'a str>,
    pub avatar_url: &'a str,
    pub signature: &'a str,
    pub created_at: Option<&'a NaiveDateTime>,
    pub updated_at: Option<&'a NaiveDateTime>,
    pub privilege: &'a u32,
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
            privilege: &self.privilege,
            show_email: &self.show_email,
            show_created_at: &self.show_created_at,
            show_updated_at: &self.show_updated_at,
        }
    }
}

impl GetSelfId for User {
    fn self_id(&self) -> &u32 { &self.id }
}

impl<'a> GetSelfId for UserRef<'a> {
    fn self_id(&self) -> &u32 { &self.id }
}

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
        self.email.as_ref().map(String::as_str).ok_or(ServiceError::BadRequest)
    }

    pub fn make_user<'a>(&'a self, id: &'a u32, hashed_password: &'a str) -> Result<NewUser<'a>, ServiceError> {
        Ok(NewUser {
            id,
            username: &self.username,
            email: self.extract_email()?,
            hashed_password,
            avatar_url: "",
            signature: "",
        })
    }
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}

#[derive(Deserialize, Debug)]
pub struct UpdateRequest {
    pub id: Option<u32>,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub signature: Option<String>,
    pub privilege: Option<u32>,
    pub show_email: Option<bool>,
    pub show_created_at: Option<bool>,
    pub show_updated_at: Option<bool>,
}

impl UpdateRequest {
    pub fn attach_id(mut self, id: Option<u32>) -> Self {
        match id {
            Some(_) => {
                self.id = id;
                self.privilege = None;
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

    pub fn make_active(id: u32) -> Self {
        UpdateRequest {
            id: Some(id),
            username: None,
            avatar_url: None,
            signature: None,
            privilege: Some(2),
            show_email: None,
            show_created_at: None,
            show_updated_at: None,
        }
    }
}

impl Validator for AuthRequest {
    fn get_username(&self) -> &str { &self.username }
    fn get_password(&self) -> &str { &self.password }
    fn get_email(&self) -> &str { self.email.as_ref().map(String::as_str).unwrap_or("") }

    fn check_self_id(&self) -> Result<(), ServiceError> { Ok(()) }
}

impl Validator for UpdateRequest {
    // ToDo: handle update validation separately.
    fn get_username(&self) -> &str {
        self.username.as_ref().map(String::as_str).unwrap_or("")
    }
    fn get_password(&self) -> &str { "" }
    fn get_email(&self) -> &str { "" }

    fn check_self_id(&self) -> Result<(), ServiceError> {
        self.id.as_ref().ok_or(ServiceError::BadRequest).map(|_| ())
    }
}