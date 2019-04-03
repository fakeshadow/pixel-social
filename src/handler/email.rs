use lettre::smtp::authentication::{Credentials, Mechanism};
use lettre::{SendableEmail, Envelope, EmailAddress, Transport, SmtpClient};
use lettre::smtp::extension::ClientId;
use lettre::smtp::ConnectionReuseParameters;

use std::env;
use std::thread;
use std::time::Duration;
use crate::model::errors::ServiceError;

// need to add redis cache
pub fn add_mail(address: String, uuid: &str) -> Result<(), ServiceError> {
    let to = match EmailAddress::new(address) {
        Ok(address) => address,
        Err(e) => {
            return Err(ServiceError::InternalServerError)
        }
    };
    //mails.zrange.add(email, uuid);
    Ok(())
}

fn send_mail() {
    let server = env::var("MAIL_SERVER").expect("Mail server must be set");
    let username = env::var("MAIL_USERNAME").expect("Mail server credentials must be set");
    let password = env::var("MAIL_PASSWORD").expect("Mail server credentials must be set");
    let domain = env::var("MAIL_DOMAIN").unwrap_or("PixelShare".to_string());

    let mut mailer = SmtpClient::new_simple(&server).unwrap()
        .hello_name(ClientId::Domain(domain.clone()))
        .credentials(Credentials::new(username, password))
        .smtp_utf8(true)
        .authentication_mechanism(Mechanism::Plain)
        .connection_reuse(ConnectionReuseParameters::ReuseUnlimited).transport();

    loop {
        // get range first and sleep if the cache is empty
        //    let to = redis.zrange.get(0,1);
        let to = "placeholder".to_string();

        let mail = SendableEmail::new(
            Envelope::new(
                Some(EmailAddress::new(domain.clone()).unwrap()),
                vec![EmailAddress::new(to).unwrap()],
            ).unwrap(),
            "id1".to_string(),
            "Hello world".to_string().into_bytes(),
        );
        match mailer.send(mail) {
            Ok(_) => {
//                redis.zrange.remove(0,1);
                return
            }
            Err(_) => {
                thread::sleep(Duration::from_secs(10));
                return
            }
        }
    }

}
