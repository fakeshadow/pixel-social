use lettre::SmtpTransport;
use uuid::Uuid;

use crate::model::user::User;

pub struct Mailer {
    pub mailer: SmtpTransport,
    pub server_url: String,
    pub self_addr: String,
    pub self_name: String,
}

pub struct Twilio {
    pub url: String,
    pub self_number: String,
    pub account_id: String,
    pub auth_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Mail {
    pub user_id: u32,
    pub username: String,
    pub uuid: String,
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SmsMessage {
    pub to: String,
    pub message: String,
}

impl Mail {
    pub fn from_user(user: &User) -> Self {
        Mail {
            user_id: user.id,
            username: user.username.to_owned(),
            uuid: Uuid::new_v4().to_string(),
            address: user.email.to_owned(),
        }
    }
}