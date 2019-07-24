use std::{env, time::Duration};

use actix::prelude::{
    ActorFuture,
    AsyncContext,
    Context,
    Future,
    WrapFuture,
};
use lettre::{
    SmtpClient,
    Transport,
    smtp::{
        ConnectionReuseParameters,
        authentication::{
            Credentials,
            Mechanism,
        },
    },
};
use lettre_email::Email;

use crate::MailService;
use crate::model::{
    mail::{Mail, Mailer, Twilio},
    errors::ServiceError,
};
use crate::handler::cache::{from_mail_queue, delete_mail_queue};

const MAIL_TIME_GAP: Duration = Duration::from_millis(1000);
const SMS_TIME_GAP: Duration = Duration::from_millis(5000);

impl MailService {
    pub fn process_mail(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(MAIL_TIME_GAP, move |act, ctx| {
            ctx.spawn(from_mail_queue(act.get_conn())
                .into_actor(act)
                .map_err(|_, _, _| ())
                .and_then(|(conn, s), act, _| {
                    // ToDo: add error handling.
                    let _ = act.send_mail(&s);
                    delete_mail_queue(&s, conn)
                        .into_actor(act)
                        .map_err(|_, _, _| ())
                        .map(|_, _, _| ())
                }));
        });
    }

    pub fn generate_mailer() -> Mailer {
        let mail_server = env::var("MAIL_SERVER").expect("Mail server must be set in .env");
        let username = env::var("MAIL_USERNAME").expect("Mail server credentials must be set  in .env");
        let password = env::var("MAIL_PASSWORD").expect("Mail server credentials must be set in .env");

        let server_url = env::var("SERVER_URL").expect("Server url must be set in .env");
        let self_addr = env::var("SELF_MAIL_ADDR").unwrap_or("Pixel@Share".to_owned());
        let self_name = env::var("SELF_MAIL_ALIAS").unwrap_or("PixelShare".to_owned());

        let mailer = SmtpClient::new_simple(&mail_server)
            .unwrap()
            .credentials(Credentials::new(username, password))
            .smtp_utf8(false)
            .authentication_mechanism(Mechanism::Plain)
            .connection_reuse(ConnectionReuseParameters::ReuseUnlimited)
            .transport();
        Mailer {
            mailer,
            server_url,
            self_addr,
            self_name,
        }
    }

    pub fn generate_twilio() -> Option<Twilio> {
        let url = env::var("TWILIO_URL").ok();
        let endpoint = env::var("TWILIO_ENDPOINT").ok();
        let account_id = env::var("TWILIO_ACCOUNT_ID").ok();
        let auth_token = env::var("TWILIO_AUTH_TOKEN").ok();

        if url.is_some() && endpoint.is_some() && account_id.is_some() && auth_token.is_some() {
            let endpoint = endpoint.unwrap();
            let account_id = account_id.unwrap();
            let auth_token = auth_token.unwrap();

            let url = format!("{}{}", endpoint, account_id);

            Some(Twilio {
                url,
                account_id,
                auth_token,
            })
        } else {
            None
        }
    }

//    pub fn process_sms(&self, ctx: &mut Context<Self>) {
//        ctx.run_interval(SMS_TIME_GAP, move |act, ctx| {
//            ctx.spawn(from_sms_queue(act.get_conn()))
//        });
//    }

    fn send_sms(&mut self) -> impl Future<Item=(), Error=ServiceError> {
        let c = awc::Client::build()
            .connector(awc::Connector::new().finish())
            .finish();

        let t = self.twilio.as_ref().unwrap();

        c.post(t.url.as_str())
            .basic_auth(t.account_id.as_str(), Some(t.auth_token.as_str()))
            .header(awc::http::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .send()
            .from_err()
            .map(|_| ())
    }

    fn send_mail(&mut self, s: &str) -> Result<(), ServiceError> {
        let mailer = &mut self.mailer;

        let mail = serde_json::from_str::<Mail>(s)?;

        let mail = Email::builder()
            .to(mail.address)
            .from((&mailer.self_addr, &mailer.self_name))
            .subject("Activate your PixelShare account")
            .alternative(format!("<p>Please click the link below </br> {}/activation/{} </p>", &mailer.server_url, mail.uuid), "Activation link")
            .build()
            .map_err(|_| ServiceError::MailServiceError)?
            .into();

        mailer
            .mailer
            .send(mail)
            .map(|_| ())
            .map_err(|_| ServiceError::MailServiceError)
    }
}

