use actix_web::{error::ResponseError, HttpResponse};
use actix::MailboxError as future_err;
use diesel::result::{DatabaseErrorKind, Error as diesel_err};

#[derive(Fail, Debug)]
pub enum ServiceError {
    #[fail(display = "Internal Server Error")]
    InternalServerError,
    #[fail(display = "BadRequest: {}", _0)]
    BadRequest(String),
    #[fail(display = "BadRequest")]
    FutureError,
    #[fail(display = "BadRequest")]
    ArcLockError,
    #[fail(display = "Forbidden")]
    UsernameTaken,
    #[fail(display = "Forbidden")]
    EmailTaken,
    #[fail(display = "BadRequest")]
    NoUser,
    #[fail(display = "Forbidden")]
    WrongPwd,
}

impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        match *self {
            ServiceError::InternalServerError => HttpResponse::InternalServerError().json("Internal Server Error"),
            ServiceError::BadRequest(ref message) => HttpResponse::BadRequest().json(message),
            ServiceError::FutureError => HttpResponse::BadRequest().json("Async error need more work"),
            ServiceError::ArcLockError => HttpResponse::BadRequest().json("Maybe Server is too busy"),
            ServiceError::UsernameTaken => HttpResponse::Forbidden().json("Username Taken"),
            ServiceError::EmailTaken => HttpResponse::Forbidden().json("Email already registered"),
            ServiceError::NoUser => HttpResponse::BadRequest().json("No user found"),
            ServiceError::WrongPwd => HttpResponse::Forbidden().json("Password is wrong"),
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

impl From<future_err> for ServiceError {
    fn from(error: future_err) -> ServiceError {
        // need to improve error handling here
        match error {
            future_err::Timeout => ServiceError::FutureError,
            future_err::Closed => ServiceError::FutureError
        }
    }
}
