use std::{env, thread, time::Duration};

use lettre::{
    EmailAddress, Envelope, SendableEmail, smtp::{authentication::{Credentials, Mechanism}, ConnectionReuseParameters, extension::ClientId}, SmtpClient,
    Transport,
};

use crate::model::{mail::Mail, errors::ServiceError};
use crate::handler::cache::MailCache;
use crate::model::common::{PoolConnectionRedis, RedisPool};

const MAIL_TIME_GAP: u64 = 500;


pub fn mail_service(pool: &RedisPool) {
    use std::{thread, time::Duration};
    let pool = pool.clone();
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(MAIL_TIME_GAP));
        match process_mail(&pool) {
            Ok(_) => (),
            Err(e) => match e {
                ServiceError::MailServiceError => {
                    println!("failed to send mail");
                    thread::sleep(Duration::from_millis(MAIL_TIME_GAP * 5))
                }
                ServiceError::NoCacheFound =>
                    thread::sleep(Duration::from_millis(MAIL_TIME_GAP * 60)),
                _ => ()
            }
        }
    });
}

fn process_mail(pool: &RedisPool) -> Result<(), ServiceError> {
    let conn = &pool.get()?;
    let cache = MailCache::from_queue(conn)?;
    match &cache {
        MailCache::GotActivation(mail) => {
            send_mail(mail)?;
            cache.remove_queue(conn)
        }
        _ => Err(ServiceError::NoCacheFound)
    }
}

fn send_mail(mail: &Mail) -> Result<(), ServiceError> {
    let url = env::var("SERVER_URL").expect("Server url must be set");
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
            Some(EmailAddress::new(domain).unwrap()),
            vec![EmailAddress::new(mail.address.to_string()).unwrap()],
        ).unwrap(),
        format!("Hello {}", mail.username),
        format!("Please visit this link to activate your account: <br> {}/activation/{}", url, mail.uuid).into_bytes(),
    );
    mailer.send(mail).map(|_| ()).map_err(|_| ServiceError::MailServiceError)
}