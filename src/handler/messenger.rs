use std::{env, time::Duration, collections::HashMap};
use futures::{
    future::{Either, ok as ft_ok},
    IntoFuture,
};

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

use crate::MessageService;
use crate::model::{
    messenger::{Mail, Mailer, Twilio, SmsMessage},
    errors::{ErrorCollection, ServiceError},
};

const MAIL_TIME_GAP: Duration = Duration::from_millis(2000);
const SMS_TIME_GAP: Duration = Duration::from_millis(1000);
const ERROR_TIME_GAP: Duration = Duration::from_millis(1000);

impl MessageService {
    pub fn start_interval(&self, ctx: &mut Context<Self>) {
        self.process_errors(ctx);
        self.process_mail(ctx);
        self.process_sms(ctx);
    }
    // errors are sent right away with sms and mail. instead of using queue.
    fn process_errors(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(ERROR_TIME_GAP, move |act, ctx| {
            if let Some(s) = act.errors.to_report().ok() {
                ctx.spawn(act
                    .send_sms_admin(s)
                    .into_actor(act)
                    .map_err(|_, _, _| ())
                    .map(|_, _, _| ()));
            };
        });
    }

    fn process_mail(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(MAIL_TIME_GAP, move |act, ctx| {
            ctx.spawn(act
                .from_queue("mail_queue")
                .into_actor(act)
                .map_err(|e, _, _| match e {
                    ServiceError::NoCache => (),
                    _ => println!("mail error is : {:?}", e)
                })
                .and_then(|s, act, _| act
                    .send_mail(s.as_str())
                    .into_future()
                    .into_actor(act)
                    .map_err(|_, _, _| ())));
        });
    }

    fn process_sms(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(SMS_TIME_GAP, move |act, ctx| {
            ctx.spawn(act
                .from_queue("sms_queue")
                .into_actor(act)
                .map_err(|e, _, _| match e {
                    ServiceError::NoCache => (),
                    _ => println!("sms error is : {:?}", e)
                })
                .and_then(|s, act, _| act
                    .send_sms_user(s.as_str())
                    .into_actor(act)
                    .map_err(|_, _, _| ())
                ));
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
            .timeout(Some(Duration::new(1, 0)))
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
        let account_id = env::var("TWILIO_ACCOUNT_ID").ok();
        let auth_token = env::var("TWILIO_AUTH_TOKEN").ok();
        let self_number = env::var("TWILIO_SELF_NUMBER").ok();

        if url.is_some() && account_id.is_some() && auth_token.is_some() && self_number.is_some() {
            Some(Twilio {
                url: url.unwrap(),
                self_number: self_number.unwrap(),
                account_id: account_id.unwrap(),
                auth_token: auth_token.unwrap(),
            })
        } else {
            None
        }
    }

    pub fn generate_errors() -> ErrorCollection {
        let is_active = env::var("USE_ERROR_SMS_REPORT").unwrap_or("".to_owned()).parse::<bool>().unwrap_or(false);

        ErrorCollection {
            is_active,
            errors: HashMap::new(),
        }
    }

    fn send_sms_admin(&mut self, msg: String) -> impl Future<Item=(), Error=ServiceError> {
        let msg = SmsMessage {
            to: self.twilio.as_ref().unwrap().self_number.to_string(),
            message: msg,
        };
        self.send_sms(msg)
    }

    fn send_sms_user(&mut self, msg: &str) -> impl Future<Item=(), Error=ServiceError> {
        let msg = match serde_json::from_str::<SmsMessage>(msg) {
            Ok(s) => s,
            Err(_) => return Either::A(ft_ok(()))
        };
        Either::B(self.send_sms(msg))
    }

    fn send_sms(&mut self, msg: SmsMessage) -> impl Future<Item=(), Error=ServiceError> {
        let t = self.twilio.as_ref().unwrap();
        let url = format!("{}{}/Messages.json", t.url.as_str(), t.account_id.as_str());

        let form = [
            ("From", t.self_number.to_string()),
            ("To", msg.to),
            ("Body", msg.message)
        ];

        let c = awc::Client::build()
            .connector(awc::Connector::new().timeout(Duration::from_secs(5)).finish())
            .finish();

        c.post(&url)
            .basic_auth(t.account_id.as_str(), Some(t.auth_token.as_str()))
            .set_header(awc::http::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .send_form(&form)
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
            .map_err(|e| {
                println!("{:?}", e);
                ServiceError::MailServiceError
            })
    }
}

