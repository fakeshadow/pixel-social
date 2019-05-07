use crate::model::user::User;

#[derive(Serialize, Deserialize)]
pub struct Mail<'a> {
    pub user_id: u32,
    pub username: &'a str,
    pub uuid: String,
    pub address: &'a str,
}

impl<'a> Mail<'a> {
    pub fn from_user(user: &'a User) -> Self {
        Mail {
            user_id: user.id,
            username: &user.username,
            uuid: "generate uuid".to_string(),
            address: &user.email,
        }
    }
}