use std::future::Future;
use std::sync::Arc;
use std::{env, time::Duration};

use futures::{lock::Mutex, FutureExt, TryFutureExt};
use hyper::{Body, Client, Request};
use hyper_tls::HttpsConnector;
use lettre::{
    smtp::{
        authentication::{Credentials, Mechanism},
        ConnectionReuseParameters,
    },
    SmtpClient, Transport,
};
use lettre_email::Email;
use redis::aio::SharedConnection;
use tokio::{future::FutureExt as TokioFutureExt, timer::Interval};

use crate::handler::cache::{CacheService, CheckRedisMut, GetQueue, GetSharedConn};
use crate::model::{
    common::dur,
    errors::{ErrorReport, RepError, ResError},
    messenger::{Mail, Mailer, SmsMessage, Twilio},
    runtime::{ChannelAddress, ChannelCreate},
    user::User,
};

const MAIL_TIME_GAP: Duration = dur(500);
const SMS_TIME_GAP: Duration = dur(500);
const ERROR_TIME_GAP: Duration = dur(60_000);
const REPORT_TIME_GAP: Duration = dur(600_000);

// handles error report, sending email and sms messages.
pub struct MessageService {
    pub url: String,
    pub cache: SharedConnection,
    pub mailer: Option<Mailer>,
    pub twilio: Option<Twilio>,
    pub error_report: ErrorReport,
}

impl ChannelCreate for MessageService {
    type Message = ErrorReport;
}

impl MessageService {
    pub(crate) async fn init(redis_url: &str) -> Result<ChannelAddress<ErrorReport>, ResError> {
        let cache = crate::handler::cache::connect_cache(redis_url)
            .await?
            .ok_or(ResError::RedisConnection)?;

        let url = redis_url.to_owned();

        let msgr = Arc::new(Mutex::new(MessageService {
            url,
            cache,
            mailer: Self::generate_mailer(),
            twilio: Self::generate_twilio(),
            error_report: Self::generate_error_report(),
        }));

        let (addr, receiver) = MessageService::create_channel();

        // ToDo: impl InjectQueue for queue and pass receiver to it.

        // spawn a future to handle mail queue.
        let msgr_mail = msgr.clone();
        tokio::spawn(async move {
            let mut interval = Interval::new_interval(MAIL_TIME_GAP);
            loop {
                interval.next().await;
                let mut msgr = msgr_mail.lock().await;
                let r = msgr.handle_mail().timeout(MAIL_TIME_GAP * 2).await;
                if let Err(e) = r {
                    // ToDo: handler error.
                    println!("mail error {:?}", e.to_string());
                }
            }
        });

        // spawn a future to handle sms queue
        let msgr_sms = msgr.clone();
        tokio::spawn(async move {
            let mut interval = Interval::new_interval(SMS_TIME_GAP);
            loop {
                interval.next().await;
                let mut msgr = msgr_sms.lock().await;
                let r = msgr.handle_sms().timeout(SMS_TIME_GAP * 2).await;
                if let Err(e) = r {
                    // ToDo: handler error.
                    println!("sms error {:?}", e.to_string());
                }
            }
        });

        // spawn a future to handle error report
        let msgr_rep = msgr.clone();
        tokio::spawn(async move {
            let mut interval = Interval::new_interval(SMS_TIME_GAP);
            loop {
                interval.next().await;
                let mut msgr = msgr_rep.lock().await;
                let r = msgr.handle_err_rep().timeout(SMS_TIME_GAP * 2).await;
                if let Err(e) = r {
                    // ToDo: handler error.
                    println!("error rep error {:?}", e.to_string());
                }
            }
        });

        Ok(addr)
    }
}

impl GetQueue for MessageService {}

impl GetSharedConn for MessageService {
    fn get_conn(&self) -> SharedConnection {
        self.cache.clone()
    }
}

impl CheckRedisMut for MessageService {
    fn self_url(&self) -> &str {
        &self.url
    }

    fn replace_redis_mut(&mut self, c: SharedConnection) {
        self.cache = c;
    }
}

impl MessageService {
    // rep errors are sent right away with sms and mail. instead of using queue.
    async fn handle_err_rep(&mut self) -> Result<(), ResError> {
        if let Ok(s) = self.error_report.stringify_report() {
            self.send_mail_admin(s.as_str())?;
            self.send_sms_admin(s.as_str()).await?;
        };
        Ok(())
    }

    // use handle_mail interval to handle reconnection to redis after connection is lost.
    async fn handle_mail(&mut self) -> Result<(), ResError> {
        let s = self
            .check_redis_mut()
            .await?
            .get_queue("mail_queue")
            .await?;

        self.send_mail_user(s.as_str())?;

        Ok(())
    }

    async fn handle_sms(&mut self) -> Result<(), ResError> {
        let s = self.get_queue("sms_queue").await?;
        self.send_sms_user(s.as_str()).await
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

    fn send_sms_admin(&mut self, msg: &str) -> impl Future<Output = Result<(), ResError>> + '_ {
        let msg = SmsMessage {
            to: self.twilio.as_ref().unwrap().self_number.to_string(),
            message: msg.to_owned(),
        };
        self.send_sms(msg)
    }

    async fn send_sms_user(&mut self, msg: &str) -> Result<(), ResError> {
        let msg = serde_json::from_str::<SmsMessage>(msg)?;

        self.send_sms(msg).await
    }

    // twilio api handler.
    async fn send_sms(&mut self, msg: SmsMessage) -> Result<(), ResError> {
        let t = self.twilio.as_ref().unwrap();
        let url = format!("{}{}/Messages.json", t.url.as_str(), t.account_id.as_str());

        let form = [
            ("From", t.self_number.to_string()),
            ("To", msg.to),
            ("Body", msg.message),
        ];

        let https = HttpsConnector::new().unwrap();
        let client = Client::builder().build::<_, Body>(https);

        let body = serde_urlencoded::to_string(form).map_err(|_| ResError::HttpClient)?;

        let auth = format!("{}:{}", t.account_id.as_str(), t.auth_token.as_str());
        let auth = base64::encode(auth.as_str());

        let req = Request::builder()
            .method(hyper::Method::POST)
            .uri(&url)
            .header(
                hyper::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .header(hyper::header::AUTHORIZATION, format!("Basic {}", auth))
            .body(Body::from(body))
            .map_err(|_| ResError::HttpClient)?;

        let res = client
            .request(req)
            .await
            .map_err(|_| ResError::HttpClient)?;
        if res.status() == 200 {
            Ok(())
        } else {
            Err(ResError::HttpClient)
        }
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
            let conn = self.get_conn();
            actix::spawn(
                CacheService::add_activation_mail_cache(conn, u.id, uuid, m)
                    .map_err(|_| ())
                    .boxed_local()
                    .compat(),
            );
        }
    }

    pub fn remove_activation_uuid(&self, uuid: &str) {
        use crate::handler::cache::DeleteCache;
        actix::spawn(self.del_cache(uuid).map_err(|_| ()).boxed_local().compat())
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
