use actix::MailboxError as future_err;
use actix_web::{error, error::ResponseError, Error, HttpResponse};
use derive_more::Display;
use diesel::result::{DatabaseErrorKind, Error as diesel_err};
use serde_json::Error as json_err;
use r2d2::Error as r2d2_err;
use r2d2_redis::redis::RedisError as redis_err;

#[derive(Debug, Display)]
pub enum ServiceError {
    #[display(fmt = "Internal Server Error")]
    InternalServerError,
    #[display(fmt = "BadRequest: {}", _0)]
    BadRequest(String),
    #[display(fmt = "BadRequest")]
    BadRequestGeneral,
    #[display(fmt = "BadRequest")]
    FutureError,
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
    #[display(fmt = "Forbidden")]
    WrongPwd,
    #[display(fmt = "Forbidden")]
    Unauthorized,
    #[display(fmt = "Forbidden")]
    AuthTimeout,
    #[display(fmt = "Forbidden")]
    NoCacheFound,
    #[display(fmt = "Internal Server Error")]
    RedisOffline,
}

impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ServiceError::InternalServerError => HttpResponse::InternalServerError().json(ErrorMessage::new("Internal Server Error")),
            ServiceError::BadRequestGeneral => HttpResponse::BadRequest().json(ErrorMessage::new("Bad Request")),
            ServiceError::BadRequest(ref message) => HttpResponse::BadRequest().json(ErrorMessage::new(message)),
            ServiceError::FutureError => HttpResponse::BadRequest().json(ErrorMessage::new("Async error need more work")),
            ServiceError::UsernameTaken => HttpResponse::BadRequest().json(ErrorMessage::new("Username Taken")),
            ServiceError::InvalidUsername => HttpResponse::BadRequest().json(ErrorMessage::new("Invalid Username")),
            ServiceError::InvalidPassword => HttpResponse::BadRequest().json(ErrorMessage::new("Invalid Password")),
            ServiceError::InvalidEmail => HttpResponse::BadRequest().json(ErrorMessage::new("Invalid Email")),
            ServiceError::EmailTaken => HttpResponse::BadRequest().json(ErrorMessage::new("Email already registered")),
            ServiceError::NotFound => HttpResponse::NotFound().json(ErrorMessage::new("Not found")),
            ServiceError::WrongPwd => HttpResponse::Forbidden().json(ErrorMessage::new("Password is wrong")),
            ServiceError::Unauthorized => HttpResponse::Forbidden().json(ErrorMessage::new("Unauthorized")),
            ServiceError::AuthTimeout => HttpResponse::Forbidden().json(ErrorMessage::new("Authentication Timeout.Please login again")),
            ServiceError::RedisOffline => HttpResponse::InternalServerError().json(ErrorMessage::new("Cache service is offline")),
            ServiceError::NoCacheFound => HttpResponse::InternalServerError().json(ErrorMessage::new("Cache not found and database is not connected"))
        }
    }
}

impl From<Error> for ServiceError {
    fn from(err: Error) -> ServiceError {
        match err {
            _ => ServiceError::BadRequest(err.to_string()),
        }
    }
}

impl From<redis_err> for ServiceError {
    fn from(_err: redis_err) -> ServiceError {
        ServiceError::InternalServerError
    }
}

impl From<r2d2_err> for ServiceError {
    fn from(err: r2d2_err) -> ServiceError {
        match err {
            _ => ServiceError::NoCacheFound,
        }
    }
}

impl From<json_err> for ServiceError {
    fn from(_err: json_err) -> ServiceError {
        ServiceError::InternalServerError
    }
}

impl From<diesel_err> for ServiceError {
    fn from(error: diesel_err) -> ServiceError {
        match error {
            diesel_err::DatabaseError(kind, info) => {
                if let DatabaseErrorKind::UniqueViolation = kind {
                    let message = info.details().unwrap_or_else(|| info.message()).to_string();
                    return ServiceError::BadRequest(message);
                }
                ServiceError::InternalServerError
            }
            _ => ServiceError::InternalServerError,
        }
    }
}

impl From<future_err> for ServiceError {
    fn from(err: future_err) -> ServiceError {
        match err {
            future_err::Timeout => ServiceError::FutureError,
            future_err::Closed => ServiceError::BadRequest(err.to_string()),
        }
    }
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
