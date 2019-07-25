use std::error::Error;

use derive_more::Display;
use actix_web::{error::ResponseError, HttpResponse};

// service errors use from trait to convert error types and generate http response or added to error report.
#[derive(Debug, Display, Eq, PartialEq, Hash)]
pub enum ServiceError {
    #[display(fmt = "Internal Server Error")]
    InternalServerError,
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
    #[display(fmt = "MailError")]
    MailServiceError,
    #[display(fmt = "RedisError")]
    RedisError,
    #[display(fmt = "Internal Server Error")]
    PARSE,
    #[display(fmt = "No Content Found")]
    NoContent,
    #[display(fmt = "No Cache Found")]
    NoCache,
    #[display(fmt = "Ids From Cache")]
    IdsFromCache(Vec<u32>),
    #[display(fmt = "Connection Time Out")]
    TimeOut,
    #[display(fmt = "Connection Error")]
    ConnectError,
    #[display(fmt = "Invalid Url")]
    InvalidUrl(String),
}

impl ResponseError for ServiceError {
    fn render_response(&self) -> HttpResponse {
        match self {
            ServiceError::InternalServerError => HttpResponse::InternalServerError().json(ErrorMessage::new("Internal Server Error")),
            ServiceError::BadRequest => HttpResponse::BadRequest().json(ErrorMessage::new("Bad Request")),
            ServiceError::BadRequestDb(e) => HttpResponse::BadRequest().json(e),
            ServiceError::NoContent => HttpResponse::NoContent().finish(),
            ServiceError::UsernameTaken => HttpResponse::BadRequest().json(ErrorMessage::new("Username already taken")),
            ServiceError::EmailTaken => HttpResponse::BadRequest().json(ErrorMessage::new("Email already registered")),
            ServiceError::InvalidUsername => HttpResponse::BadRequest().json(ErrorMessage::new("Invalid Username")),
            ServiceError::InvalidPassword => HttpResponse::BadRequest().json(ErrorMessage::new("Invalid Password")),
            ServiceError::InvalidEmail => HttpResponse::BadRequest().json(ErrorMessage::new("Invalid Email")),
            ServiceError::WrongPwd => HttpResponse::Forbidden().json(ErrorMessage::new("Password is wrong")),
            ServiceError::Unauthorized => HttpResponse::Forbidden().json(ErrorMessage::new("Unauthorized")),
            ServiceError::AuthTimeout => HttpResponse::Forbidden().json(ErrorMessage::new("Authentication Timeout.Please login again")),
            ServiceError::PARSE => HttpResponse::InternalServerError().json(ErrorMessage::new("Parsing error")),
            ServiceError::NotActive => HttpResponse::Forbidden().json(ErrorMessage::new("User is not activated yet")),
            ServiceError::Blocked => HttpResponse::Forbidden().json(ErrorMessage::new("User is blocked")),
            _ => HttpResponse::InternalServerError().json(ErrorMessage::new("Unknown")),
        }
    }
}

impl From<awc::error::SendRequestError> for ServiceError {
    fn from(e: awc::error::SendRequestError) -> ServiceError {
        use awc::error::SendRequestError;
        match e {
            SendRequestError::Url(i) => ServiceError::InvalidUrl(i.to_string()),
            SendRequestError::Connect(_) => ServiceError::ConnectError,
            SendRequestError::Timeout => ServiceError::TimeOut,
            _ => ServiceError::InternalServerError
        }
    }
}

impl From<tokio_postgres::error::Error> for ServiceError {
    fn from(e: tokio_postgres::error::Error) -> ServiceError {
        ServiceError::BadRequestDb(DatabaseErrorMessage {
            category: None,
            description: e.description().to_string(),
        })
    }
}

impl<T> From<(tokio_postgres::error::Error, T)> for ServiceError {
    fn from(e: (tokio_postgres::error::Error, T)) -> ServiceError {
        ServiceError::BadRequestDb(DatabaseErrorMessage {
            category: None,
            description: e.0.description().to_owned(),
        })
    }
}

impl From<actix::MailboxError> for ServiceError {
    fn from(e: actix::MailboxError) -> ServiceError {
        match e {
            actix::MailboxError::Closed => ServiceError::BadRequest,
            actix::MailboxError::Timeout => ServiceError::InternalServerError
        }
    }
}

impl From<redis::RedisError> for ServiceError {
    fn from(e: redis::RedisError) -> ServiceError {
        if e.is_connection_dropped() || e.is_connection_refusal() || e.is_timeout() {
            return ServiceError::RedisError;
        }
        ServiceError::InternalServerError
    }
}

impl From<serde_json::Error> for ServiceError {
    fn from(_err: serde_json::Error) -> ServiceError {
        ServiceError::InternalServerError
    }
}

impl From<std::num::ParseIntError> for ServiceError {
    fn from(_err: std::num::ParseIntError) -> ServiceError {
        ServiceError::PARSE
    }
}

impl From<chrono::format::ParseError> for ServiceError {
    fn from(_err: chrono::format::ParseError) -> ServiceError {
        ServiceError::PARSE
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

use std::collections::HashMap;

// error collection is passed to messenger service actor.
#[derive(Debug)]
pub struct ErrorCollection {
    pub is_active: bool,
    pub errors: HashMap<ServiceError, u32>,
}

impl ErrorCollection {
    pub fn to_report(&mut self) -> Result<String, ()> {
        if self.is_active {
            let mut message = String::from("Got error:");

            let err = &mut self.errors;

            if let Some(v) = err.get_mut(&ServiceError::MailServiceError) {
                if *v > 2 {
                    message.push_str(" MailServiceError(Could be email server offline)");
                    *v = 0;
                }
            }
            if !message.ends_with(":") {
                Ok(message)
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }
}