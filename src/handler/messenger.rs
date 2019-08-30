use std::{cell::RefCell, env, time::Duration};

use actix::prelude::{Actor, ActorFuture, Addr, AsyncContext, Context, Future, WrapFuture};
use actix_web::client;
use futures01::{
    future::{ok as ft_ok, Either},
    IntoFuture,
};
use lettre::{
    smtp::{
        authentication::{Credentials, Mechanism},
        ConnectionReuseParameters,
    },
    SmtpClient, Transport,
};
use lettre_email::Email;
use redis::aio::SharedConnection;

use crate::handler::cache::{CacheService, CheckCacheConn, GetQueue, GetSharedConn};
use crate::model::{
    errors::{ErrorReport, RepError, ResError},
    messenger::{Mail, Mailer, SmsMessage, Twilio},
    user::User,
};

const MAIL_TIME_GAP: Duration = Duration::from_millis(500);
const SMS_TIME_GAP: Duration = Duration::from_millis(500);
const ERROR_TIME_GAP: Duration = Duration::from_secs(60);
const REPORT_TIME_GAP: Duration = Duration::from_secs(600);

// actor handles error report, sending email and sms messages.
pub struct MessageService {
    pub url: String,
    pub cache: RefCell<SharedConnection>,
    pub mailer: Option<Mailer>,
    pub twilio: Option<Twilio>,
    pub error_report: ErrorReport,
}

impl Actor for MessageService {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.start_interval(ctx);
    }
}

impl MessageService {
    pub(crate) async fn init(redis_url: &str) -> Result<Addr<MessageService>, ResError> {
        let cache = crate::handler::cache::connect_cache(redis_url)
            .await?
            .ok_or(ResError::RedisConnection)?;

        let url = redis_url.to_owned();

        Ok(MessageService::create(move |_| MessageService {
            url,
            cache: RefCell::new(cache),
            mailer: Self::generate_mailer(),
            twilio: Self::generate_twilio(),
            error_report: Self::generate_error_report(),
        }))
    }
}

impl GetQueue for MessageService {}

impl GetSharedConn for MessageService {
    fn get_conn(&self) -> SharedConnection {
        self.cache.borrow().clone()
    }
}

impl CheckCacheConn for MessageService {
    fn self_url(&self) -> String {
        self.url.to_owned()
    }

    fn replace_cache(&self, c: SharedConnection) {
        self.cache.replace(c);
    }
}

impl MessageService {
    pub fn start_interval(&self, ctx: &mut Context<Self>) {
        if self.mailer.is_some() {
            self.process_mail(ctx);
        }
        if self.twilio.is_some() {
            self.process_sms(ctx);
        }
        if self.error_report.use_report {
            self.process_errors(ctx);
        }
    }
    // rep errors are sent right away with sms and mail. instead of using queue.
    fn process_errors(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(ERROR_TIME_GAP, move |act, ctx| {
            if let Ok(s) = act.error_report.stringify_report() {
                let f1 = act.send_sms_admin(s.as_str());
                let f2 = act.send_mail_admin(s.as_str());

                ctx.spawn(f1.join(f2).map_err(|_| ()).map(|_| ()).into_actor(act));
            };
        });
    }

    // use process email interval to handle reconnection to redis after connection is lost.
    fn process_mail(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(MAIL_TIME_GAP, move |act, ctx| {
            ctx.spawn(
                act.check_cache_conn_01()
                    .into_actor(act)
                    .and_then(|opt, act, _| {
                        act.if_replace_cache(opt)
                            .get_queue_01("mail_queue")
                            .into_actor(act)
                            .and_then(|s, act, _| {
                                act.send_mail_user(s.as_str()).into_future().into_actor(act)
                            })
                    })
                    .map_err(|e, act, _| {
                        act.add_err_to_rep(RepError::from(e));
                    }),
            );
        });
    }

    fn process_sms(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(SMS_TIME_GAP, move |act, ctx| {
            ctx.spawn(
                act.get_queue_01("sms_queue")
                    .into_actor(act)
                    .and_then(|s, act, _| act.send_sms_user(s.as_str()).into_actor(act))
                    .map_err(|e, act, _| {
                        act.add_err_to_rep(RepError::from(e));
                    }),
            );
        });
    }

    pub fn generate_mailer() -> Option<Mailer> {
        let mail_server = env::var("MAIL_SERVER").expect("Mail server must be set in .env");
        let username =
            env::var("MAIL_USERNAME").expect("Mail server credentials must be set  in .env");
        let password =
            env::var("MAIL_PASSWORD").expect("Mail server credentials must be set in .env");

        let server_url = env::var("SERVER_URL").expect("Server url must be set in .env");
        let self_addr = env::var("SELF_MAIL_ADDR").unwrap_or_else(|_| "Pixel@Share".to_owned());
        let self_name = env::var("SELF_MAIL_ALIAS").unwrap_or_else(|_| "PixelShare".to_owned());

        match SmtpClient::new_simple(&mail_server) {
            Ok(m) => {
                let mailer = m
                    .timeout(Some(Duration::new(1, 0)))
                    .credentials(Credentials::new(username, password))
                    .smtp_utf8(false)
                    .authentication_mechanism(Mechanism::Plain)
                    .connection_reuse(ConnectionReuseParameters::ReuseUnlimited)
                    .transport();
                Some(Mailer {
                    mailer,
                    server_url,
                    self_addr,
                    self_name,
                })
            }
            Err(_) => None,
        }
    }

    pub fn generate_twilio() -> Option<Twilio> {
        let url = env::var("TWILIO_URL").ok();
        let account_id = env::var("TWILIO_ACCOUNT_ID").ok();
        let auth_token = env::var("TWILIO_AUTH_TOKEN").ok();
        let self_number = env::var("TWILIO_SELF_NUMBER").ok();

        if let Some(url) = url {
            if let Some(account_id) = account_id {
                if let Some(auth_token) = auth_token {
                    if let Some(self_number) = self_number {
                        return Some(Twilio {
                            url,
                            self_number,
                            account_id,
                            auth_token,
                        });
                    }
                }
            }
        }

        None
    }

    pub fn generate_error_report() -> ErrorReport {
        let use_report = env::var("USE_ERROR_SMS_REPORT")
            .unwrap_or_else(|_| "false".to_owned())
            .parse::<bool>()
            .unwrap_or(false);

        ErrorReport {
            use_report,
            reports: hashbrown::HashMap::new(),
            last_report_time: std::time::Instant::now(),
        }
    }

    fn send_sms_admin(&mut self, msg: &str) -> impl Future<Item = (), Error = ResError> {
        let msg = SmsMessage {
            to: self.twilio.as_ref().unwrap().self_number.to_string(),
            message: msg.to_owned(),
        };
        self.send_sms(msg)
    }

    fn send_sms_user(&mut self, msg: &str) -> impl Future<Item = (), Error = ResError> {
        let msg = match serde_json::from_str::<SmsMessage>(msg) {
            Ok(s) => s,
            Err(_) => return Either::A(ft_ok(())),
        };
        Either::B(self.send_sms(msg))
    }

    // twilio api handler.
    fn send_sms(&mut self, msg: SmsMessage) -> impl Future<Item = (), Error = ResError> {
        let t = self.twilio.as_ref().unwrap();
        let url = format!("{}{}/Messages.json", t.url.as_str(), t.account_id.as_str());

        let form = [
            ("From", t.self_number.to_string()),
            ("To", msg.to),
            ("Body", msg.message),
        ];

        let c = client::Client::build()
            .connector(
                client::Connector::new()
                    .timeout(Duration::from_secs(5))
                    .finish(),
            )
            .finish();

        c.post(&url)
            .basic_auth(t.account_id.as_str(), Some(t.auth_token.as_str()))
            .set_header(
                actix_web::http::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .send_form(&form)
            .from_err()
            .map(|_| ())
    }

    fn send_mail_admin(&mut self, rep: &str) -> Result<(), ResError> {
        let mail = Mail::ErrorReport { report: rep };
        self.send_mail(&mail)
    }

    fn send_mail_user(&mut self, s: &str) -> Result<(), ResError> {
        let mail = serde_json::from_str::<Mail>(s)?;

        self.send_mail(&mail)
    }

    fn send_mail(&mut self, mail: &Mail) -> Result<(), ResError> {
        let mailer = self.mailer.as_mut().unwrap();

        let (to, subject, html, text) = match *mail {
            Mail::Activation { to, uuid } => (
                to,
                "Activate your PixelShare account",
                format!(
                    "<p>Please click the link below </br> {}/activation/{} </p>",
                    &mailer.server_url, uuid
                ),
                "Activation link",
            ),
            Mail::ErrorReport { report } => (
                mailer.self_addr.as_str(),
                "Error Report",
                report.to_owned(),
                "",
            ),
        };

        let mail = Email::builder()
            .to(to)
            .from((mailer.self_addr.as_str(), mailer.self_name.as_str()))
            .subject(subject)
            .alternative(html.as_str(), text)
            .build()?
            .into();

        Ok(mailer.mailer.send(mail).map(|_| ())?)
    }
}

impl CacheService {
    pub fn add_activation_mail(&self, u: User) {
        let uuid = uuid::Uuid::new_v4().to_string();
        let mail = Mail::new_activation(u.email.as_str(), uuid.as_str());

        if let Ok(m) = serde_json::to_string(&mail) {
            actix::spawn(self.add_activation_mail_01(u.id, uuid, m).map_err(|_| ()));
        }
    }

    pub fn remove_activation_uuid(&self, uuid: &str) {
        use crate::handler::cache::DeleteCache;
        actix::spawn(self.del_cache_01(uuid).map_err(|_| ()))
    }
}

impl MessageService {
    fn add_err_to_rep(&mut self, e: RepError) {
        match self.error_report.reports.get_mut(&e) {
            Some(v) => {
                *v += 1;
            }
            None => {
                self.error_report.reports.insert(e, 1);
            }
        }
    }
}

impl ErrorReport {
    pub fn stringify_report(&mut self) -> Result<String, ()> {
        if self.use_report {
            let now = chrono::Utc::now().naive_utc();
            let mut message = format!("Time: {}%0aGot erros:", now);

            let rep = &mut self.reports;

            if let Some(v) = rep.get_mut(&RepError::Redis) {
                if *v > 2 {
                    message
                        .push_str("%0aRedis Service Error(Could be redis server offline/IO error)");
                }
                *v = 0;
            }
            if let Some(v) = rep.get_mut(&RepError::Database) {
                if *v > 2 {
                    message.push_str(
                        "%0aDatabase Service Error(Could be database server offline/IO error)",
                    );
                }
                *v = 0;
            }

            if let Some(v) = rep.get_mut(&RepError::SMS) {
                if *v > 2 {
                    message
                        .push_str("%0aSMS Service Error(Could be lost connection to twilio API)");
                }
                *v = 0;
            }
            if let Some(v) = rep.get_mut(&RepError::MailBuilder) {
                if *v > 3 {
                    message.push_str("%0aMail Service Error(Can not build email)");
                }
                *v = 0;
            }
            if let Some(v) = rep.get_mut(&RepError::MailTransport) {
                if *v > 2 {
                    message.push_str("%0aMail Service Error(Can not transport email. Could be email server offline)");
                }
                *v = 0;
            }
            if let Some(v) = rep.get_mut(&RepError::HttpClient) {
                if *v > 3 {
                    message.push_str(
                        "%0aHttp Client Error(Could be network issue with target API entry)",
                    );
                }
                *v = 0;
            }
            if !message.ends_with(':')
                && std::time::Instant::now().duration_since(self.last_report_time) > REPORT_TIME_GAP
            {
                Ok(message)
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }
}
