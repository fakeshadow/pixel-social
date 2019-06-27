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
use lettre_email::Email;

use crate::model::{
    mail::Mail,
    errors::ServiceError,
    actors::{
        SharedConn,
        MailService,
    },
};
use crate::handler::cache::{process_mail};

const MAIL_TIME_GAP: Duration = Duration::from_millis(2000);

impl MailService {
    pub fn hb(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(MAIL_TIME_GAP, move |act, ctx| {
            ctx.wait(process_mail(act.cache.as_ref().unwrap().clone(), send_mail)
                .into_actor(act)
                .map_err(|_, _, _| ()));
        });
    }
}

fn send_mail(mail: &Mail) -> Result<(), ServiceError> {
    let url = env::var("SERVER_URL").expect("Server url must be set in .env");
    let server = env::var("MAIL_SERVER").expect("Mail server must be set in .env");
    let self_username = env::var("MAIL_USERNAME").expect("Mail server credentials must be set  in .env");
    let password = env::var("MAIL_PASSWORD").expect("Mail server credentials must be set in .env");
    let domain = env::var("MAIL_DOMAIN").unwrap_or("PixelShare".to_owned());

    let mut mailer = SmtpClient::new_simple(&server).map_err(|_| ServiceError::MailServiceError)?
        .hello_name(ClientId::Domain(domain.clone()))
        .credentials(Credentials::new(self_username, password))
        .smtp_utf8(true)
        .authentication_mechanism(Mechanism::Plain)
        .connection_reuse(ConnectionReuseParameters::ReuseUnlimited).transport();

    let mail = SendableEmail::new(
        Envelope::new(
            Some(EmailAddress::new(domain).unwrap()),
            vec![EmailAddress::new(mail.address.to_owned()).unwrap()],
        ).unwrap(),
        format!("Hello {}", mail.username),
        format!("Please visit this link to activate your account: <br> {}/activation/{}", url, mail.uuid).into_bytes(),
    );
    mailer.send(mail).map(|_| ()).map_err(|_| ServiceError::MailServiceError)
}