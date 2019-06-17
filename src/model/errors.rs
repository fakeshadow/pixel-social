use std::error::Error;

use derive_more::Display;
use actix_web::{error::BlockingError, error::ResponseError, HttpResponse};
use diesel::result::{DatabaseErrorKind, Error as DieselError};

#[derive(Debug, Display)]
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
    //    #[display(fmt = "BadRequest")]
//    NotFound,
    #[display(fmt = "RedisError: {}", _0)]
    RedisError(String),
    #[display(fmt = "Forbidden")]
    WrongPwd,
    #[display(fmt = "Forbidden")]
    Unauthorized,
    #[display(fmt = "Forbidden")]
    AuthTimeout,
    #[display(fmt = "MailError")]
    MailServiceError,
    #[display(fmt = "Internal Server Error")]
    PARSEINT,
}

impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ServiceError::InternalServerError => HttpResponse::InternalServerError().json(ErrorMessage::new("Internal Server Error")),
            ServiceError::BadRequest => HttpResponse::BadRequest().json(ErrorMessage::new("Bad Request")),
            ServiceError::BadRequestDb(e) => HttpResponse::BadRequest().json(e),
            ServiceError::UsernameTaken => HttpResponse::BadRequest().json(ErrorMessage::new("Username already taken")),
            ServiceError::EmailTaken => HttpResponse::BadRequest().json(ErrorMessage::new("Email already registered")),
            ServiceError::InvalidUsername => HttpResponse::BadRequest().json(ErrorMessage::new("Invalid Username")),
            ServiceError::InvalidPassword => HttpResponse::BadRequest().json(ErrorMessage::new("Invalid Password")),
            ServiceError::InvalidEmail => HttpResponse::BadRequest().json(ErrorMessage::new("Invalid Email")),
//            ServiceError::NotFound => HttpResponse::NotFound().json(ErrorMessage::new("Not found")),
            ServiceError::WrongPwd => HttpResponse::Forbidden().json(ErrorMessage::new("Password is wrong")),
            ServiceError::Unauthorized => HttpResponse::Forbidden().json(ErrorMessage::new("Unauthorized")),
            ServiceError::AuthTimeout => HttpResponse::Forbidden().json(ErrorMessage::new("Authentication Timeout.Please login again")),
            ServiceError::RedisError(e) => HttpResponse::InternalServerError().json(ErrorMessage::new(e)),
            ServiceError::PARSEINT => HttpResponse::InternalServerError().json(ErrorMessage::new("Parsing int error")),
            _ => HttpResponse::InternalServerError().json(ErrorMessage::new("Unknown")),
        }
    }
    fn render_response(&self) -> HttpResponse {
        self.error_response()
    }
}

impl From<tokio_postgres::error::Error> for ServiceError {
    fn from(e: tokio_postgres::error::Error) -> ServiceError {
        ServiceError::BadRequestDb(DatabaseErrorMessage {
            message: e.description().to_owned(),
            details: Some(e.to_string()),
            hint: None,
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
        ServiceError::BadRequestDb(DatabaseErrorMessage {
            message: e.category().to_owned(),
            details: Some(e.description().to_owned()),
            hint: None,
        })
    }
}

impl From<BlockingError<ServiceError>> for ServiceError {
    fn from(err: BlockingError<ServiceError>) -> ServiceError {
        match err {
            BlockingError::Error(e) => e,
            _ => ServiceError::InternalServerError
        }
    }
}


// ToDo: remove r2d2 redis impl
use r2d2_redis::redis::{RedisError, ErrorKind as RedisErrorKind};

impl From<RedisError> for ServiceError {
    fn from(err: RedisError) -> ServiceError {
        match err.kind() {
            RedisErrorKind::ResponseError => ServiceError::RedisError(err.to_string()),
            RedisErrorKind::IoError => ServiceError::RedisError(err.to_string()),
            _ => ServiceError::InternalServerError
        }
    }
}

impl From<DieselError> for ServiceError {
    fn from(err: DieselError) -> ServiceError {
        match err {
            DieselError::DatabaseError(kind, info) => match kind {
                DatabaseErrorKind::UniqueViolation =>
                    ServiceError::BadRequestDb(DatabaseErrorMessage {
                        message: info.message().to_string(),
                        details: info.details().map(|i| i.to_string()),
                        hint: info.hint().map(|i| i.to_string()),
                    }),
                _ => ServiceError::InternalServerError
            }
            _ => ServiceError::InternalServerError,
        }
    }
}

impl From<r2d2::Error> for ServiceError {
    fn from(_err: r2d2::Error) -> ServiceError { ServiceError::InternalServerError }
}

impl From<serde_json::Error> for ServiceError {
    fn from(_err: serde_json::Error) -> ServiceError {
        ServiceError::InternalServerError
    }
}

impl From<std::num::ParseIntError> for ServiceError {
    fn from(_err: std::num::ParseIntError) -> ServiceError {
        ServiceError::PARSEINT
    }
}

impl From<chrono::format::ParseError> for ServiceError {
    fn from(_err: chrono::format::ParseError) -> ServiceError {
        ServiceError::InternalServerError
    }
}

#[derive(Serialize, Debug)]
pub struct DatabaseErrorMessage {
    message: String,
    details: Option<String>,
    hint: Option<String>,
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
