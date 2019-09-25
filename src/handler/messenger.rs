use std::{env, future::Future, pin::Pin, sync::Arc, time::Duration};

use futures::{channel::mpsc::UnboundedReceiver, lock::Mutex as FutMutex, FutureExt, TryFutureExt};
use hashbrown::HashMap;
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

use crate::handler::cache::{CacheService, CheckRedisMut, GetQueue, GetSharedConn};
use crate::model::runtime::{SendRepError, SpawnIntervalHandler, SpawnQueueHandler};
use crate::model::{
    common::dur,
    errors::{RepError, ResError},
    messenger::{Mail, Mailer, SmsMessage, Twilio},
    runtime::{ChannelAddress, ChannelCreate},
    user::User,
};

const REPORT_INTERVAL: Duration = dur(600_000);
const REPORT_TIMEOUT: Duration = dur(30_000);

const MAIL_INTERVAL: Duration = dur(500);
const MAIL_TIMEOUT: Duration = dur(5_000);

const SMS_INTERVAL: Duration = dur(500);
const SMS_TIMEOUT: Duration = dur(5_000);

pub type RepErrorAddr = ChannelAddress<RepError>;

type ReportQueue = Arc<FutMutex<HashMap<RepError, u32>>>;
type SharedMessageService = Arc<FutMutex<MessageService>>;

// handles error report, sending email and sms messages.
pub struct MessageService {
    pub url: String,
    pub cache: SharedConnection,
    pub mailer: Option<Mailer>,
    pub twilio: Option<Twilio>,
    pub queue: ReportQueue,
    pub rep_addr: RepErrorAddr,
}

impl ChannelCreate for MessageService {
    type Message = RepError;
}

impl MessageService {
    pub(crate) async fn init(
        redis_url: &str,
        use_sms: bool,
        use_mail: bool,
        use_rep: bool,
    ) -> Result<RepErrorAddr, ResError> {
        let cache = crate::handler::cache::connect_cache(redis_url)
            .await?
            .ok_or(ResError::RedisConnection)?;

        let url = redis_url.to_owned();

        let (addr, receiver) = MessageService::create_channel();

        let (queue, handler) = ReportQueueHandler::new(receiver);

        handler.spawn_handle();

        let msg = Arc::new(FutMutex::new(MessageService {
            url,
            cache,
            mailer: Self::generate_mailer(),
            twilio: Self::generate_twilio(),
            queue,
            rep_addr: addr.clone(),
        }));

        // spawn a future to handle mail queue(mail queue lives in redis so no addition channel is needed).
        if use_mail {
            MailerInterval::from(msg.clone()).spawn_interval(MAIL_INTERVAL, MAIL_TIMEOUT);
        }

        // spawn a future to handle sms queue
        if use_sms {
            SMSInterval::from(msg.clone()).spawn_interval(SMS_INTERVAL, SMS_TIMEOUT);
        }

        // spawn a future to handle error report
        if use_rep {
            if !use_sms && !use_mail {
                panic!("Error report need at least Email or SMS service to function. Please check .env setting");
            }

            ErrorReport::from(msg.clone()).spawn_interval(REPORT_INTERVAL, REPORT_TIMEOUT);
        }

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

struct MailerInterval(SharedMessageService);

impl From<SharedMessageService> for MailerInterval {
    fn from(m: SharedMessageService) -> MailerInterval {
        MailerInterval(m)
    }
}

impl SpawnIntervalHandler for MailerInterval {
    fn handle<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = Result<(), ResError>> + Send + 'a>> {
        Box::pin(async move {
            let mut msg = self.0.lock().await;
            msg.handle_mail().await
        })
    }
}

impl SendRepError for MailerInterval {
    fn send_err_rep<'a>(
        &'a mut self,
        e: ResError,
    ) -> Pin<Box<dyn Future<Output = Result<(), ResError>> + Send + 'a>> {
        Box::pin(async move {
            let mail = self.0.lock().await;
            mail.rep_addr.do_send(e.into());
            Ok(())
        })
    }
}

struct SMSInterval(SharedMessageService);

impl From<SharedMessageService> for SMSInterval {
    fn from(m: SharedMessageService) -> SMSInterval {
        SMSInterval(m)
    }
}

impl SpawnIntervalHandler for SMSInterval {
    fn handle<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = Result<(), ResError>> + Send + 'a>> {
        Box::pin(async move {
            let mut msg = self.0.lock().await;
            msg.handle_sms().await
        })
    }
}

impl SendRepError for SMSInterval {
    fn send_err_rep<'a>(
        &'a mut self,
        e: ResError,
    ) -> Pin<Box<dyn Future<Output = Result<(), ResError>> + Send + 'a>> {
        Box::pin(async move {
            let sms = self.0.lock().await;
            sms.rep_addr.do_send(e.into());
            Ok(())
        })
    }
}

struct ErrorReport(SharedMessageService);

impl From<SharedMessageService> for ErrorReport {
    fn from(m: SharedMessageService) -> ErrorReport {
        ErrorReport(m)
    }
}

impl SpawnIntervalHandler for ErrorReport {
    fn handle<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = Result<(), ResError>> + Send + 'a>> {
        Box::pin(async move {
            let mut msg = self.0.lock().await;
            msg.handle_err_rep().await
        })
    }
}

impl SendRepError for ErrorReport {}

impl MessageService {
    // rep errors are sent right away with sms and mail. instead of using queue.
    async fn handle_err_rep(&mut self) -> Result<(), ResError> {
        if let Ok(s) = self.stringify_report().await {
            self.send_mail_admin(s.as_str())?;
            self.send_sms_admin(s.as_str()).await?;
        };
        Ok(())
    }

    async fn stringify_report(&mut self) -> Result<String, ()> {
        let now = chrono::Utc::now().naive_utc();
        let mut message = format!("Time: {}%0aGot erros:", now);

        let mut queue = self.queue.lock().await;

        if let Some(v) = queue.get_mut(&RepError::Redis) {
            if *v > 2 {
                message.push_str("%0aRedis Service Error(Could be redis server offline/IO error)");
            }
            *v = 0;
        }
        if let Some(v) = queue.get_mut(&RepError::Database) {
            if *v > 2 {
                message.push_str(
                    "%0aDatabase Service Error(Could be database server offline/IO error)",
                );
            }
            *v = 0;
        }
        if let Some(v) = queue.get_mut(&RepError::Mailer) {
            if *v > 3 {
                message.push_str("%0aMail Service Error(Can not build or send email)");
            }
            *v = 0;
        }
        if let Some(v) = queue.get_mut(&RepError::HttpClient) {
            if *v > 3 {
                message
                    .push_str("%0aHttp Client Error(Could be network issue with target API entry)");
            }
            *v = 0;
        }
        if !message.ends_with(':') {
            Ok(message)
        } else {
            Err(())
        }
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

#[derive(Debug)]
struct ReportQueueHandler {
    queue: ReportQueue,
    receiver: UnboundedReceiver<RepError>,
}

impl SpawnQueueHandler<RepError> for ReportQueueHandler {
    type Queue = ReportQueue;
    type Error = ();

    fn new(receiver: UnboundedReceiver<RepError>) -> (Self::Queue, Self) {
        let queue = Arc::new(FutMutex::new(HashMap::new()));
        let handler = ReportQueueHandler {
            queue: queue.clone(),
            receiver,
        };

        (queue, handler)
    }

    fn receiver(&mut self) -> &mut UnboundedReceiver<RepError> {
        &mut self.receiver
    }

    fn handle_message<'a>(
        &'a mut self,
        msg: RepError,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>> {
        Box::pin(async move {
            let mut queue = self.queue.lock().await;
            match queue.get_mut(&msg) {
                Some(v) => {
                    *v += 1;
                }
                None => {
                    queue.insert(msg, 1);
                }
            };
            Ok(())
        })
    }
}
