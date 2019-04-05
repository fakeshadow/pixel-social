use actix::Message;
use chrono::NaiveDateTime;

use crate::model::common::{GetSelfId, Validator};
use crate::model::errors::ServiceError;
use crate::schema::users;

#[derive(Queryable, Insertable, Serialize, Debug)]
#[table_name = "users"]
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

#[derive(Queryable, Serialize, Deserialize, Clone, Debug)]
pub struct SlimUser {
	pub id: u32,
	pub username: String,
	pub email: String,
	pub avatar_url: String,
	pub signature: String,
	pub created_at: NaiveDateTime,
	pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
	pub id: u32,
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

pub struct AuthRequest<'a> {
	pub username: &'a str,
	pub password: &'a str,
	pub email: &'a str,
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

#[derive(Deserialize, Clone)]
pub struct UserUpdateJson {
	pub id: Option<u32>,
	pub username: Option<String>,
	pub password: Option<String>,
	pub email: Option<String>,
	pub avatar_url: Option<String>,
	pub signature: Option<String>,
	pub is_admin: Option<u32>,
	pub blocked: Option<bool>,
}

pub struct UserUpdateRequest<'a> {
	pub id: Option<&'a u32>,
	pub username: Option<&'a String>,
	pub avatar_url: Option<&'a String>,
	pub signature: Option<&'a String>,
	pub is_admin: Option<&'a u32>,
	pub blocked: Option<&'a bool>,
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

impl<'a> UserUpdateRequest<'a> {
	pub fn update_user_data(&self, user: &'a mut User) -> Result<&'a User,()> {
		if let Some(new_username) = self.username {
			user.username = new_username.to_string()
		};
		if let Some(new_avatar_url) = self.avatar_url {
			user.avatar_url = new_avatar_url.to_string()
		};
		if let Some(new_signature) = self.signature {
			user.signature = new_signature.to_string()
		};
		if let Some(new_is_admin) = self.is_admin {
			user.is_admin = *new_is_admin
		};
		if let Some(new_blocked) = self.blocked {
			user.blocked = *new_blocked
		};
		Ok(user)
	}
}

impl<'a> User {
	pub fn new(
		id: u32,
		username: &'a str,
		email: &'a str,
		hashed_password: &'a str,
	) -> NewUser<'a> {
		NewUser {
			id,
			username,
			email,
			hashed_password,
			// change to default avatar url later
			avatar_url: "",
			signature: "",
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
	fn get_self_id(&self) -> &u32 {
		&self.id
	}
	fn get_self_id_copy(&self) -> u32 {
		self.id
	}
}

pub enum UserQuery<'a> {
	Register(AuthRequest<'a>),
	Login(AuthRequest<'a>),
	GetMe(&'a u32),
	GetUser(&'a str),
	UpdateUser(UserUpdateRequest<'a>),
}

pub enum UserQueryResult {
	Registered,
	LoggedIn(AuthResponse),
	GotUser(User),
	GotSlimUser(SlimUser),
}
