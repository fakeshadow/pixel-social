use actix_web::{error::ResponseError, HttpResponse};
use actix::MailboxError as future_err;
use diesel::result::{DatabaseErrorKind, Error as diesel_err};

#[derive(Fail, Debug)]
pub enum ServiceError {
    #[fail(display = "Internal Server Error")]
    InternalServerError,

    #[fail(display = "BadRequest: {}", _0)]
    BadRequest(String),

    #[fail(display = "QueryConflict: {}", _0)]
    QueryConflict(String),
}

impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        match *self {
            ServiceError::InternalServerError => {
                HttpResponse::InternalServerError().json("Internal Server Error")
            }
            ServiceError::BadRequest(ref message) => HttpResponse::BadRequest().json(message),
            ServiceError::QueryConflict(ref message) => HttpResponse::Forbidden().json(message),
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
        ServiceError::InternalServerError
    }
}
