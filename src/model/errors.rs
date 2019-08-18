use std::error::Error;

use actix_web::{error::ResponseError, HttpResponse};
use derive_more::{Display, From};
use psn_api_rs::PSNError;

// res errors use from trait to convert error types and generate http response or added to error report.
#[derive(Debug, Display, From)]
pub enum ResError {
    #[display(fmt = "Internal Server Error")]
    InternalServerError,
    #[display(fmt = "Not Found")]
    NotFound,
    #[display(fmt = "Postgres Read Error")]
    DataBaseReadError,
    #[display(fmt = "Redis Connection Error")]
    RedisConnection,
    #[display(fmt = "BadRequest")]
    BadRequestDb(DatabaseErrorMessage),
    #[display(fmt = "BadRequest")]
    BadRequest,
    #[display(fmt = "BadRequest")]
    UsernameTaken,
    #[display(fmt = "BadRequest")]
    EmailTaken,
    #[display(fmt = "BadRequest")]
    InvalidUsername,
    #[display(fmt = "BadRequest")]
    InvalidPassword,
    #[display(fmt = "BadRequest")]
    InvalidEmail,
    #[display(fmt = "Forbidden")]
    WrongPwd,
    #[display(fmt = "Forbidden")]
    Unauthorized,
    #[display(fmt = "Forbidden")]
    NotActive,
    #[display(fmt = "Forbidden")]
    Blocked,
    #[display(fmt = "Forbidden")]
    AuthTimeout,
    #[display(fmt = "Internal Server Error")]
    ParseError,
    #[display(fmt = "No Content Found")]
    NoContent,
    #[display(fmt = "No Cache Found")]
    NoCache,
    #[display(fmt = "Ids From Cache")]
    IdsFromCache(Vec<u32>),
}

impl ResponseError for ResError {
    fn render_response(&self) -> HttpResponse {
        match self {
            ResError::InternalServerError => {
                HttpResponse::InternalServerError().json(ErrorMessage::new("Internal Server Error"))
            }
            ResError::BadRequest => {
                HttpResponse::BadRequest().json(ErrorMessage::new("Bad Request"))
            }
            ResError::BadRequestDb(e) => HttpResponse::BadRequest().json(e),
            ResError::NoContent => HttpResponse::NoContent().finish(),
            ResError::UsernameTaken => {
                HttpResponse::BadRequest().json(ErrorMessage::new("Username already taken"))
            }
            ResError::EmailTaken => {
                HttpResponse::BadRequest().json(ErrorMessage::new("Email already registered"))
            }
            ResError::InvalidUsername => {
                HttpResponse::BadRequest().json(ErrorMessage::new("Invalid Username"))
            }
            ResError::InvalidPassword => {
                HttpResponse::BadRequest().json(ErrorMessage::new("Invalid Password"))
            }
            ResError::InvalidEmail => {
                HttpResponse::BadRequest().json(ErrorMessage::new("Invalid Email"))
            }
            ResError::WrongPwd => {
                HttpResponse::Forbidden().json(ErrorMessage::new("Password is wrong"))
            }
            ResError::Unauthorized => {
                HttpResponse::Forbidden().json(ErrorMessage::new("Unauthorized"))
            }
            ResError::AuthTimeout => HttpResponse::Forbidden().json(ErrorMessage::new(
                "Authentication Timeout.Please login again",
            )),
            ResError::ParseError => {
                HttpResponse::InternalServerError().json(ErrorMessage::new("Parsing error"))
            }
            ResError::NotActive => {
                HttpResponse::Forbidden().json(ErrorMessage::new("User is not activated yet"))
            }
            ResError::Blocked => {
                HttpResponse::Forbidden().json(ErrorMessage::new("User is blocked"))
            }
            _ => HttpResponse::InternalServerError().json(ErrorMessage::new("Unknown")),
        }
    }
}

impl ResError {
    pub fn stringify(&self) -> &'static str {
        match self {
            ResError::NotFound => "Not Found",
            _ => "Internal Server Error",
        }
    }
}

impl From<tokio_postgres::error::Error> for ResError {
    fn from(e: tokio_postgres::error::Error) -> ResError {
        println!("{:?}", e);
        ResError::BadRequestDb(DatabaseErrorMessage {
            category: None,
            description: e.description().to_string(),
        })
    }
}

impl From<actix::MailboxError> for ResError {
    fn from(e: actix::MailboxError) -> ResError {
        match e {
            actix::MailboxError::Closed => ResError::BadRequest,
            actix::MailboxError::Timeout => ResError::InternalServerError,
        }
    }
}

impl From<redis::RedisError> for ResError {
    fn from(e: redis::RedisError) -> ResError {
        if e.is_connection_dropped() || e.is_connection_refusal() || e.is_timeout() {
            ResError::RedisConnection
        } else {
            ResError::InternalServerError
        }
    }
}

impl From<serde_json::Error> for ResError {
    fn from(_err: serde_json::Error) -> ResError {
        ResError::InternalServerError
    }
}

impl From<std::num::ParseIntError> for ResError {
    fn from(_err: std::num::ParseIntError) -> ResError {
        ResError::ParseError
    }
}

impl From<chrono::format::ParseError> for ResError {
    fn from(_err: chrono::format::ParseError) -> ResError {
        ResError::ParseError
    }
}

//ToDo: handle psn error.
impl From<PSNError> for ResError {
    fn from(_e: PSNError) -> ResError {
        ResError::InternalServerError
    }
}

#[derive(Serialize, Debug, Eq, PartialEq, Hash)]
pub struct DatabaseErrorMessage {
    category: Option<String>,
    description: String,
}

#[derive(Serialize)]
struct ErrorMessage<'a> {
    msg: Option<&'a str>,
    error: &'a str,
}

impl<'a> ErrorMessage<'a> {
    fn new(msg: &'a str) -> Self {
        ErrorMessage {
            msg: None,
            error: msg,
        }
    }
}

// report error will be sent to users by sms/email/message
#[derive(Debug, Display, Hash, Eq, PartialEq)]
pub enum RepError {
    Ignore,
    JsonIO,
    Database,
    Redis,
    MailBuilder,
    MailTransport,
    SMS,
    HttpClient,
}

impl From<awc::error::SendRequestError> for RepError {
    fn from(_e: awc::error::SendRequestError) -> RepError {
        RepError::HttpClient
    }
}

impl From<ResError> for RepError {
    fn from(e: ResError) -> RepError {
        match e {
            ResError::RedisConnection => RepError::Redis,
            ResError::DataBaseReadError => RepError::Database,
            _ => RepError::Ignore,
        }
    }
}

impl From<serde_json::Error> for RepError {
    fn from(e: serde_json::Error) -> RepError {
        if e.is_io() {
            return RepError::JsonIO;
        }
        RepError::Ignore
    }
}

#[derive(Debug)]
pub struct ErrorReport {
    pub use_report: bool,
    pub reports: hashbrown::HashMap<RepError, u32>,
    pub last_report_time: std::time::Instant,
}
