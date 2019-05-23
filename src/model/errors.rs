use derive_more::Display;
use actix_http::Response;
use actix_web::{error::BlockingError, error::ResponseError, HttpResponse};
use diesel::result::{DatabaseErrorKind, Error as DieselError};


#[derive(Debug, Display)]
pub enum ServiceError {
    #[display(fmt = "Internal Server Error")]
    InternalServerError,
    #[display(fmt = "BadRequest: {}", _0)]
    BadRequest(String),
    #[display(fmt = "BadRequest")]
    BadRequestDb(DatabaseErrorMessage),
    #[display(fmt = "BadRequest")]
    BadRequestGeneral,
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
    #[display(fmt = "BadRequest")]
    NotFound,
    #[display(fmt = "RedisError: {}", _0)]
    RedisError(String),
    #[display(fmt = "RedisError")]
    RedisErrorGeneral,
    #[display(fmt = "Forbidden")]
    WrongPwd,
    #[display(fmt = "Forbidden")]
    Unauthorized,
    #[display(fmt = "Forbidden")]
    AuthTimeout,
    #[display(fmt = "Forbidden")]
    NoCacheFound,
    #[display(fmt = "Internal Server Error")]
    CacheOffline,
    #[display(fmt = "MailError")]
    MailServiceError,
}

impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ServiceError::InternalServerError => HttpResponse::InternalServerError().json(ErrorMessage::new("Internal Server Error")),
            ServiceError::BadRequestGeneral => HttpResponse::BadRequest().json(ErrorMessage::new("Bad Request")),
            ServiceError::BadRequest(message) => HttpResponse::BadRequest().json(ErrorMessage::new(message)),
            ServiceError::BadRequestDb(e) => HttpResponse::BadRequest().json(e),
            ServiceError::UsernameTaken => HttpResponse::BadRequest().json(ErrorMessage::new("Username Taken")),
            ServiceError::InvalidUsername => HttpResponse::BadRequest().json(ErrorMessage::new("Invalid Username")),
            ServiceError::InvalidPassword => HttpResponse::BadRequest().json(ErrorMessage::new("Invalid Password")),
            ServiceError::InvalidEmail => HttpResponse::BadRequest().json(ErrorMessage::new("Invalid Email")),
            ServiceError::EmailTaken => HttpResponse::BadRequest().json(ErrorMessage::new("Email already registered")),
            ServiceError::NotFound => HttpResponse::NotFound().json(ErrorMessage::new("Not found")),
            ServiceError::WrongPwd => HttpResponse::Forbidden().json(ErrorMessage::new("Password is wrong")),
            ServiceError::Unauthorized => HttpResponse::Forbidden().json(ErrorMessage::new("Unauthorized")),
            ServiceError::AuthTimeout => HttpResponse::Forbidden().json(ErrorMessage::new("Authentication Timeout.Please login again")),
            ServiceError::NoCacheFound => HttpResponse::InternalServerError().json(ErrorMessage::new("Cache not found")),
            ServiceError::CacheOffline => HttpResponse::InternalServerError().json(ErrorMessage::new("Cache service is offline")),
            _ => HttpResponse::InternalServerError().json(ErrorMessage::new("Unknown")),
        }
    }
    fn render_response(&self) -> Response {
        self.error_response()
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

use r2d2_redis::redis::{RedisError, ErrorKind as RedisErrorKind};

impl From<RedisError> for ServiceError {
    fn from(err: RedisError) -> ServiceError {
        match err.kind() {
            RedisErrorKind::ResponseError => ServiceError::RedisError(err.to_string()),
            RedisErrorKind::IoError => ServiceError::RedisError(err.to_string()),
            _ => ServiceError::RedisErrorGeneral
        }
    }
}

impl From<DieselError> for ServiceError {
    fn from(e: DieselError) -> ServiceError {
        match e {
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

use r2d2::Error as R2d2Error;

impl From<R2d2Error> for ServiceError {
    fn from(_err: R2d2Error) -> ServiceError { ServiceError::InternalServerError }
}


use serde_json::Error as SerdeError;

impl From<SerdeError> for ServiceError {
    fn from(_e: SerdeError) -> ServiceError {
        ServiceError::InternalServerError
    }
}

use chrono::format::ParseError as ParseNavDateError;

impl From<ParseNavDateError> for ServiceError {
    fn from(_e: ParseNavDateError) -> ServiceError {
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
