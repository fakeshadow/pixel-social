use std::{env, time::Duration};

use actix::prelude::*;
use lettre::{
    EmailAddress,
    Envelope,
    SendableEmail,
    SmtpClient,
    Transport,
    SmtpTransport,
    smtp::{
        ConnectionReuseParameters,
        authentication::{
            Credentials,
            Mechanism},
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
use crate::handler::cache::{from_mail_queue, delete_mail_queue};

const MAIL_TIME_GAP: Duration = Duration::from_millis(2000);

impl MailService {
    pub fn hb(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(MAIL_TIME_GAP, move |act, ctx| {
            ctx.wait(from_mail_queue(act.cache.as_ref().unwrap().clone())
                .into_actor(act)
                .map_err(|_, _, _| ())
                .and_then(|(conn, s), act, _| {
                    let _ = send_mail(act.mailer.as_mut().unwrap(), &s);
                    delete_mail_queue(&s, conn)
                        .into_actor(act)
                        .map_err(|_, _, _| ())
                        .map(|_, _, _| ())
                }));
        });
    }
}

pub fn generate_mailer() -> Option<SmtpTransport> {
    let mail_server = env::var("MAIL_SERVER").expect("Mail server must be set in .env");
    let username = env::var("MAIL_USERNAME").expect("Mail server credentials must be set  in .env");
    let password = env::var("MAIL_PASSWORD").expect("Mail server credentials must be set in .env");

    let mailer = SmtpClient::new_simple(&mail_server)
        .unwrap()
        .credentials(Credentials::new(username, password))
        .smtp_utf8(false)
        .authentication_mechanism(Mechanism::Plain)
        .connection_reuse(ConnectionReuseParameters::ReuseUnlimited)
        .transport();
    Some(mailer)
}

fn send_mail(mailer: &mut SmtpTransport, s: &str) -> Result<(), ServiceError> {
    let mail = serde_json::from_str::<Mail>(s)?;

    let server_url = env::var("SERVER_URL").expect("Server url must be set in .env");
    let self_email = env::var("SELF_MAIL_ADDR").unwrap_or("Pixel@Share".to_owned());
    let self_name = env::var("SELF_MAIL_ALIAS").unwrap_or("PixelShare".to_owned());

    let mail = Email::builder()
        .to(mail.address)
        .from((self_email, self_name))
        .subject("Activate your PixelShare account")
        .alternative(format!("<p>Please click the link below </br> {}/activation/{} </p>", server_url, mail.uuid), "Activation link")
        .build()
        .map_err(|_|ServiceError::MailServiceError)?
        .into();

    mailer.send(mail)
        .map(|_| ())
        .map_err(|_| ServiceError::MailServiceError)
}