use lettre::SmtpTransport;

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
pub enum Mail<'a> {
    Activation { to: &'a str, uuid: &'a str },
    ErrorReport { report: &'a str },
}

impl<'a> Mail<'a> {
    pub fn new_activation(to: &'a str, uuid: &'a str) -> Self {
        Mail::Activation { to, uuid }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SmsMessage {
    pub to: String,
    pub message: String,
}
