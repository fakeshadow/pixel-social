use std::{env, time::Duration};

use actix::prelude::*;
use redis::cmd;
use lettre::{
    EmailAddress,
    Envelope,
    SendableEmail,
    SmtpClient,
    Transport,
    smtp::{
        ConnectionReuseParameters,
        extension::ClientId,
        authentication::{Credentials, Mechanism},
    },
};

use crate::model::{
    mail::Mail,
    errors::ServiceError,
    actors::{
        SharedConn,
        MailService,
    },
};
use crate::handler::cache::{from_mail_queue, remove_mail_queue};

const MAIL_TIME_GAP: Duration = Duration::from_millis(1000);

impl MailService {
    pub fn hb(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(MAIL_TIME_GAP, move |act, ctx| {
//            let _ = process_mail(act.cache.as_ref().unwrap().clone());
        });
    }
}

fn process_mail(
    conn: SharedConn
) -> impl Future<Item=(), Error=ServiceError> {
    from_mail_queue(conn)
        .and_then(|(conn, m)| {
            let _ = send_mail(&m);
            remove_mail_queue(conn)
        })
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