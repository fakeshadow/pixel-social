use std::{env, future::Future, time::Duration};

use actix_send::prelude::*;
use futures::FutureExt;
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

use crate::handler::cache::{pool_redis, MyRedisPool};
use crate::model::{
    common::dur,
    errors::{RepError, ResError},
    messenger::{Mail, Mailer, SmsMessage, Twilio},
    user::User,
};

const REPORT_INTERVAL: Duration = dur(600_000);
const MAIL_INTERVAL: Duration = dur(500);
const SMS_INTERVAL: Duration = dur(500);

// MailerTask is an actor runs a interval and read from redis cache and send mails to users.
// It would also receive admin message and send it immediately.
#[actor]
struct MailerService {
    mailer: Option<Mailer>,
}

pub struct AdminMailMsg(String);

#[handler_v2]
impl MailerService {
    async fn handle_msg(&mut self, msg: AdminMailMsg) {
        let _ = self.handle_mail_admin(msg.0.as_str());
    }
}

impl MailerService {
    fn generate_mailer(mut self) -> Self {
        let mail_server = env::var("MAIL_SERVER").expect("Mail server must be set in .env");
        let username =
            env::var("MAIL_USERNAME").expect("Mail server credentials must be set  in .env");
        let password =
            env::var("MAIL_PASSWORD").expect("Mail server credentials must be set in .env");

        let server_url = env::var("SERVER_URL").expect("Server url must be set in .env");
        let self_addr = env::var("SELF_MAIL_ADDR").unwrap_or_else(|_| "Pixel@Share".to_owned());
        let self_name = env::var("SELF_MAIL_ALIAS").unwrap_or_else(|_| "PixelShare".to_owned());

        let mailer = SmtpClient::new_simple(&mail_server)
            .unwrap_or_else(|e| panic!("Failed to establish SmtpClient. Error is: {:?}", e))
            .timeout(Some(Duration::new(1, 0)))
            .credentials(Credentials::new(username, password))
            .smtp_utf8(false)
            .authentication_mechanism(Mechanism::Plain)
            .connection_reuse(ConnectionReuseParameters::ReuseUnlimited)
            .transport();

        self.mailer = Some(Mailer {
            mailer,
            server_url,
            self_addr,
            self_name,
        });

        self
    }

    async fn handle_mail_user(&mut self) -> Result<(), ResError> {
        let s = pool_redis().get_queue("mail_queue").await?;
        let mail = serde_json::from_str::<Mail>(s.as_str())?;
        self.send_mail(&mail)
    }

    fn handle_mail_admin(&mut self, rep: &str) -> Result<(), ResError> {
        let mail = Mail::ErrorReport { report: rep };
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

// SMSService is an actor runs a interval task and read from redis cache and send sms message to
// users.
// It would also receive message and sent to admin through sms immediately.
#[actor]
struct SMSService {
    twilio: Option<Twilio>,
}

pub struct AdminSMSMsg(String);

#[handler_v2]
impl SMSService {
    async fn handle_msg(&mut self, msg: AdminSMSMsg) {
        let _ = self.handle_sms_admin(msg.0.as_str()).await;
    }
}

impl SMSService {
    fn generate_twilio(mut self) -> Self {
        let url = env::var("TWILIO_URL").expect("TWILIO_URL must be set in .env");
        let account_id =
            env::var("TWILIO_ACCOUNT_ID").expect("TWILIO_ACCOUNT_ID must be set in .env");
        let auth_token =
            env::var("TWILIO_AUTH_TOKEN").expect("TWILIO_AUTH_TOKEN must be set in .env");
        let self_number =
            env::var("TWILIO_SELF_NUMBER").expect("TWILIO_SELF_NUMBER must be set in .env");

        self.twilio = Some(Twilio {
            url,
            self_number,
            account_id,
            auth_token,
        });

        self
    }

    async fn handle_sms_user(&mut self) -> Result<(), ResError> {
        let s = pool_redis().get_queue("sms_queue").await?;
        let msg = serde_json::from_str::<SmsMessage>(s.as_str())?;
        self.send_sms(msg).await
    }

    fn handle_sms_admin(&mut self, msg: &str) -> impl Future<Output = Result<(), ResError>> + '_ {
        let msg = SmsMessage {
            to: self.twilio.as_ref().unwrap().self_number.to_string(),
            message: msg.to_owned(),
        };
        self.send_sms(msg)
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

        let https = HttpsConnector::new();
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
}

// ErrReportTask run on a interval and handle Error Report.
// At the beginning of every interval we try to pop a message from the task's context and convert it to RepError which will be inserted to self.error HashMap.
// Then we go through the HashMap and stringify the errors beyond threshold and send them to MailerTask and SMSTask in String form.
#[actor]
pub struct ErrReportService {
    mailer_addr: Option<Address<MailerService>>,
    sms_addr: Option<Address<SMSService>>,
    error: HashMap<RepError, u32>,
}

pub struct ErrReportMsg(pub ResError);

#[handler_v2]
impl ErrReportService {
    async fn handle_msg(&mut self, msg: ErrReportMsg) {
        let e = msg.0.into();
        match self.error.get_mut(&e) {
            Some(v) => {
                *v += 1;
            }
            None => {
                self.error.insert(e, 1);
            }
        };
    }
}

impl ErrReportService {
    async fn handle_err_rep(&mut self) -> Result<(), ResError> {
        if let Ok(s) = self.stringify_report() {
            if let Some(addr) = self.mailer_addr.as_ref() {
                let _ = addr.send(AdminMailMsg(s.to_owned())).await;
            };
            if let Some(addr) = self.sms_addr.as_ref() {
                let _ = addr.send(AdminSMSMsg(s)).await;
            };
        };
        Ok(())
    }

    fn stringify_report(&mut self) -> Result<String, ()> {
        let now = chrono::Utc::now().naive_utc();
        let mut message = format!("Time: {}%0aGot erros:", now);

        let queue = &mut self.error;

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
}

pub(crate) type ErrReportServiceAddr = Address<ErrReportService>;

pub(crate) async fn init_message_services(
    use_mail: bool,
    use_sms: bool,
    use_rep: bool,
) -> Option<ErrReportServiceAddr> {
    let mailer_addr = if use_mail {
        let builder = MailerService::builder(|| async {
            let actor = MailerService { mailer: None };
            actor.generate_mailer()
        });

        let addr: Address<MailerService> = builder.start().await;

        addr.run_interval(MAIL_INTERVAL, |mailer| {
            Box::pin(async move {
                // ToDo: handle error.
                let _ = mailer.handle_mail_user().await;
            })
        })
        .await
        .expect("Failed to start MailerService interval task");

        Some(addr)
    } else {
        None
    };

    let sms_addr = if use_sms {
        let builder = SMSService::builder(|| async {
            let actor = SMSService { twilio: None };
            actor.generate_twilio()
        });

        let addr: Address<SMSService> = builder.start().await;

        addr.run_interval(SMS_INTERVAL, |sms| {
            Box::pin(async move {
                // ToDo: handle error.
                let _ = sms.handle_sms_user().await;
            })
        })
        .await
        .expect("Failed to start SMSService interval task");

        Some(addr)
    } else {
        None
    };

    let err_rep_addr = if use_rep {
        let builder = ErrReportService::builder(move || {
            let mailer_addr = mailer_addr.clone();
            let sms_addr = sms_addr.clone();
            async {
                ErrReportService {
                    mailer_addr,
                    sms_addr,
                    error: Default::default(),
                }
            }
        });

        let addr: Address<ErrReportService> = builder.start().await;

        addr.run_interval(REPORT_INTERVAL, |rep| {
            Box::pin(async move {
                // ToDo: handle error.
                let _ = rep.handle_err_rep().await;
            })
        })
        .await
        .expect("Failed to start SMSService interval task");

        Some(addr)
    } else {
        None
    };

    err_rep_addr
}

impl MyRedisPool {
    pub(crate) async fn add_activation_mail(&self, u: Vec<User>) {
        if let Some(u) = u.first() {
            let uuid = uuid::Uuid::new_v4().to_string();
            let mail = Mail::new_activation(u.email.as_str(), uuid.as_str());

            if let Ok(m) = serde_json::to_string(&mail) {
                if let Ok(pool_ref) = self.get().await {
                    let conn = (&*pool_ref).clone();
                    actix_rt::spawn(
                        Self::add_activation_mail_cache(conn, u.id, uuid, m).map(|_| ()),
                    );
                }
            }
        }
    }

    pub(crate) async fn remove_activation_uuid(&self, uuid: &str) {
        if let Ok(pool_ref) = self.get().await {
            let conn = (&*pool_ref).clone();
            actix_rt::spawn(Self::del_cache(conn, uuid.to_owned()).map(|_| ()));
        }
    }
}
