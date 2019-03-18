use chrono::{NaiveDateTime, Local};

use crate::schema::users;

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable)]
#[table_name = "users"]
pub struct User {
    pub uid: u32,
    pub username: String,
    pub email: String,
    pub password: String,
    pub avatar_url: String,
    pub signature: String,
    pub created_at: NaiveDateTime,
    pub is_admin: bool,
    pub blocked: bool,
}

#[derive(Debug, Serialize)]
pub struct SlimUser {
    pub uid: u32,
    pub username: String,
    pub email: String,
    pub avatar_url: String,
    pub signature: String,
}

#[derive(Debug, Deserialize)]
pub struct IncomingRegister {
    pub username: String,
    pub password: String,
    pub email: String,
}

#[derive(Debug)]
pub struct RegisterData {
    pub uid: u32,
    pub username: String,
    pub password: String,
    pub email: String,
}

#[derive(Debug)]
pub struct RegisterCheck {
    pub username: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginData {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoggedInData {
    pub token: String,
    pub user_data: SlimUser
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
            is_admin: false,
            blocked: false
        }
    }
    pub fn slim(self) -> SlimUser {
        SlimUser {
            uid: self.uid,
            username: self.username,
            email: self.email,
            avatar_url: self.avatar_url,
            signature: self.signature,
        }
    }
}

