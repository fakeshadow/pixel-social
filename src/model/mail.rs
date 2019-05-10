use crate::model::user::User;

#[derive(Serialize, Deserialize)]
pub struct Mail {
    pub user_id: u32,
    pub username: String,
    pub uuid: String,
    pub address: String,
}

impl Mail {
    pub fn from_user(user: &User) -> Self {
        Mail {
            user_id: user.id,
            username: user.username.to_string(),
            uuid: "generate uuid".to_string(),
            address: user.email.to_string(),
        }
    }
}