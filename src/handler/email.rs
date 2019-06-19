use actix::prelude::*;

use std::{env, time::Duration};

use lettre::{
    EmailAddress, Envelope, SendableEmail, smtp::{authentication::{Credentials, Mechanism}, ConnectionReuseParameters, extension::ClientId}, SmtpClient,
    Transport,
};

use crate::model::{mail::Mail, errors::ServiceError};
use crate::model::common::{PoolConnectionRedis, RedisPool};

const MAIL_TIME_GAP: Duration = Duration::from_millis(1000);

pub struct MailService {
    pool: RedisPool,
}

impl Actor for MailService {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
    }
}

impl MailService {
    pub fn init(pool: RedisPool) -> Self {
        MailService {
            pool,
        }
    }

    fn hb(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(MAIL_TIME_GAP, |act, ctx| {
//            let _ = process_mail(&act.pool);
        });
    }
}

//fn process_mail(pool: &RedisPool) -> Result<(), ServiceError> {
//    let conn = &pool.get()?;
//    let cache = MailCache::from_queue(conn)?;
//    match &cache {
//        MailCache::GotActivation(mail) => {
//            send_mail(mail)?;
//            cache.remove_queue(conn)
//        }
//        _ => Err(ServiceError::InternalServerError)
//    }
//}

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