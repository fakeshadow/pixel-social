use std::{env, thread, time::Duration};

use lettre::{
    EmailAddress, Envelope, SendableEmail, smtp::{authentication::{Credentials, Mechanism}, ConnectionReuseParameters, extension::ClientId}, SmtpClient,
    Transport,
};

use crate::model::{mail::Mail, errors::ServiceError};

pub fn send_mail(mail: Mail) -> Result<(), ServiceError> {
    let url = "http://test.com";
    let server = env::var("MAIL_SERVER").expect("Mail server must be set");
    let self_username = env::var("MAIL_USERNAME").expect("Mail server credentials must be set");
    let password = env::var("MAIL_PASSWORD").expect("Mail server credentials must be set");
    let domain = env::var("MAIL_DOMAIN").unwrap_or("PixelShare".to_string());

    let mut mailer = SmtpClient::new_simple(&server).unwrap()
        .hello_name(ClientId::Domain(domain.clone()))
        .credentials(Credentials::new(self_username, password))
        .smtp_utf8(true)
        .authentication_mechanism(Mechanism::Plain)
        .connection_reuse(ConnectionReuseParameters::ReuseUnlimited).transport();

    let mail = SendableEmail::new(
        Envelope::new(
            Some(EmailAddress::new(domain.clone()).unwrap()),
            vec![EmailAddress::new(mail.address.to_string()).unwrap()],
        ).unwrap(),
        format!("Hello {}", mail.username),
        format!("Please visit this link to activate your account: <br> http://{}/activation/{}", url, mail.uuid).into_bytes(),
    );
    mailer.send(mail).map(|_| ()).map_err(|_| ServiceError::MailServiceError)
}
