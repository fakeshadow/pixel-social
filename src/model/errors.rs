use actix_web::{error::ResponseError, error::MultipartError as multi_err, HttpResponse};
use actix::MailboxError as future_err;
use diesel::result::{DatabaseErrorKind, Error as diesel_err};

use r2d2::Error as r2d2_err;
use r2d2_redis::redis::RedisError as redis_err;

#[derive(Fail, Debug)]
pub enum ServiceError {
    #[fail(display = "Internal Server Error")]
    InternalServerError,
    #[fail(display = "BadRequest: {}", _0)]
    BadRequest(String),
    #[fail(display = "BadRequest")]
    BadRequestGeneral,
    #[fail(display = "BadRequest")]
    FutureError,
    #[fail(display = "BadRequest")]
    UsernameTaken,
    #[fail(display = "BadRequest")]
    EmailTaken,
    #[fail(display = "BadRequest")]
    UsernameShort,
    #[fail(display = "BadRequest")]
    NotFound,
    #[fail(display = "Forbidden")]
    WrongPwd,
    #[fail(display = "Forbidden")]
    Unauthorized,
    #[fail(display = "Forbidden")]
    AuthTimeout,
    #[fail(display = "Forbidden")]
    NoCacheFound,
    #[fail(display = "Internal Server Error")]
    RedisOffline,
    #[fail(display = "IBadRequest")]
    RegisterLimit
}


impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ServiceError::InternalServerError => HttpResponse::InternalServerError().json(ErrorMessage::new("Internal Server Error")),
            ServiceError::BadRequestGeneral => HttpResponse::BadRequest().json(ErrorMessage::new("Bad Request")),
            ServiceError::BadRequest(ref message) => HttpResponse::BadRequest().json(ErrorMessage::new(message)),
            ServiceError::FutureError => HttpResponse::BadRequest().json(ErrorMessage::new("Async error need more work")),
            ServiceError::UsernameTaken => HttpResponse::BadRequest().json(ErrorMessage::new("Username Taken")),
            ServiceError::UsernameShort => HttpResponse::BadRequest().json(ErrorMessage::new("Username Too Short")),
            ServiceError::EmailTaken => HttpResponse::BadRequest().json(ErrorMessage::new("Email already registered")),
            ServiceError::NotFound => HttpResponse::NotFound().json(ErrorMessage::new("Not found")),
            ServiceError::WrongPwd => HttpResponse::Forbidden().json(ErrorMessage::new("Password is wrong")),
            ServiceError::Unauthorized => HttpResponse::Forbidden().json(ErrorMessage::new("Unauthorized")),
            ServiceError::AuthTimeout => HttpResponse::Forbidden().json(ErrorMessage::new("Authentication Timeout.Please login again")),
            ServiceError::RedisOffline => HttpResponse::InternalServerError().json(ErrorMessage::new("Cache service is offline")),
            ServiceError::NoCacheFound => HttpResponse::InternalServerError().json(ErrorMessage::new("Cache not found and database is not connected")),
            ServiceError::RegisterLimit => HttpResponse::BadRequest().json(ErrorMessage::new("Register requirement not met")),
        }
    }
}

impl From<multi_err> for ServiceError {
    fn from(error: multi_err) -> ServiceError {
        match error {
            multi_err::Payload(a) => {
                return ServiceError::BadRequestGeneral;
            }
            _ => ServiceError::InternalServerError
        }
    }
}

impl From<redis_err> for ServiceError {
    fn from(err: redis_err) -> ServiceError {
        println!("{:?}", err);
        ServiceError::InternalServerError
    }
}

impl From<r2d2_err> for ServiceError {
    fn from(err: r2d2_err) -> ServiceError {
        match err {
            _ => ServiceError::NoCacheFound
        }
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
            _ => ServiceError::InternalServerError
        }
    }
}

//use std::option::NoneError as none_err;
//
//impl From<none_err> for ServiceError {
//    fn from(err: none_err) -> ServiceError {
//        ServiceError::GotNone
//    }
//}


impl From<future_err> for ServiceError {
    fn from(error: future_err) -> ServiceError {
        // need to improve error handling here
        match error {
            future_err::Timeout => ServiceError::FutureError,
            future_err::Closed => ServiceError::FutureError
        }
    }
}

#[derive(Serialize)]
struct ErrorMessage<'a> {
    error: &'a str,
}

impl<'a> ErrorMessage<'a> {
    fn new(msg: &'a str) -> Self {
        ErrorMessage {
            error: msg
        }
    }
}