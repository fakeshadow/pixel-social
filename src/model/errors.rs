use std::error::Error;

use derive_more::Display;
use actix_web::{error::ResponseError, HttpResponse};

// res errors use from trait to convert error types and generate http response or added to error report.
#[derive(Debug, Display)]
pub enum ResError {
    #[display(fmt = "Internal Server Error")]
    InternalServerError,
    #[display(fmt = "Fail Getting Rows from DB")]
    DataBaseReadError,
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
            ResError::InternalServerError => HttpResponse::InternalServerError().json(ErrorMessage::new("Internal Server Error")),
            ResError::BadRequest => HttpResponse::BadRequest().json(ErrorMessage::new("Bad Request")),
            ResError::BadRequestDb(e) => HttpResponse::BadRequest().json(e),
            ResError::DataBaseReadError => HttpResponse::InternalServerError().json(ErrorMessage::new("Database Reading Error. Data could be corrupted")),
            ResError::NoContent => HttpResponse::NoContent().finish(),
            ResError::UsernameTaken => HttpResponse::BadRequest().json(ErrorMessage::new("Username already taken")),
            ResError::EmailTaken => HttpResponse::BadRequest().json(ErrorMessage::new("Email already registered")),
            ResError::InvalidUsername => HttpResponse::BadRequest().json(ErrorMessage::new("Invalid Username")),
            ResError::InvalidPassword => HttpResponse::BadRequest().json(ErrorMessage::new("Invalid Password")),
            ResError::InvalidEmail => HttpResponse::BadRequest().json(ErrorMessage::new("Invalid Email")),
            ResError::WrongPwd => HttpResponse::Forbidden().json(ErrorMessage::new("Password is wrong")),
            ResError::Unauthorized => HttpResponse::Forbidden().json(ErrorMessage::new("Unauthorized")),
            ResError::AuthTimeout => HttpResponse::Forbidden().json(ErrorMessage::new("Authentication Timeout.Please login again")),
            ResError::ParseError => HttpResponse::InternalServerError().json(ErrorMessage::new("Parsing error")),
            ResError::NotActive => HttpResponse::Forbidden().json(ErrorMessage::new("User is not activated yet")),
            ResError::Blocked => HttpResponse::Forbidden().json(ErrorMessage::new("User is blocked")),
            _ => HttpResponse::InternalServerError().json(ErrorMessage::new("Unknown")),
        }
    }
}

impl From<tokio_postgres::error::Error> for ResError {
    fn from(e: tokio_postgres::error::Error) -> ResError {
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
            actix::MailboxError::Timeout => ResError::InternalServerError
        }
    }
}

impl From<redis::RedisError> for ResError {
    fn from(_e: redis::RedisError) -> ResError {
        ResError::InternalServerError
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

#[derive(Serialize, Debug, Eq, PartialEq, Hash)]
pub struct DatabaseErrorMessage {
    category: Option<String>,
    description: String,
}

#[derive(Serialize)]
struct ErrorMessage<'a> {
    error: &'a str,
}

impl<'a> ErrorMessage<'a> {
    fn new(msg: &'a str) -> Self {
        ErrorMessage { error: msg }
    }
}

// report error will be sent to users by sms/email/message
#[derive(Debug, Display, Hash, Eq, PartialEq)]
pub enum RepError {
    Ignore,
    JsonIO,
    Database,
    MailBuilder,
    MailTransport,
    SMS,
    Redis,
    HttpClient,
}

impl From<awc::error::SendRequestError> for RepError {
    fn from(_e: awc::error::SendRequestError) -> RepError {
        RepError::HttpClient
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
}