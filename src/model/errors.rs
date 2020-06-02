use std::convert::From;
use std::fmt::{Debug, Display};

use actix_web::error::BlockingError;
use actix_web::{error::ResponseError, HttpResponse};
use derive_more::{Display, From};
use psn_api_rs::psn::PSNError;
use redis_tang::RedisPoolError;
use tokio_postgres_tang::PostgresPoolError;

// res errors use from trait to convert error types and generate http response or added to error report.
#[derive(Debug, Display, From)]
pub enum ResError {
    #[display(fmt = "Internal Server Error")]
    InternalServerError,
    #[display(fmt = "Internal Server Error: {}", _0)]
    InternalServerErrorExplained(String),
    #[display(fmt = "BadRequest")]
    BadRequest,
    #[display(fmt = "BadRequest: {}", _0)]
    BadRequestExplained(String),
    #[display(fmt = "Postgres Read Error")]
    PostgresError,
    #[display(fmt = "DataBase Error: {}", _0)]
    PostgresExplained(String),
    #[display(fmt = "Not Found")]
    NotFound,
    #[display(fmt = "Redis Error")]
    RedisError,
    #[display(fmt = "Redis Error: {}", _0)]
    RedisErrorExplained(String),
    #[display(fmt = "BadRequest: Username is Taken")]
    UsernameTaken,
    #[display(fmt = "BadRequest: Email is Taken")]
    EmailTaken,
    #[display(fmt = "BadRequest: Invalid Username")]
    InvalidUsername,
    #[display(fmt = "BadRequest: Invalid Password")]
    InvalidPassword,
    #[display(fmt = "BadRequest: Invalid Email Address")]
    InvalidEmail,
    #[display(fmt = "Forbidden: Wrong Password")]
    WrongPwd,
    #[display(fmt = "Forbidden: Unauthorized")]
    Unauthorized,
    #[display(fmt = "NotActive")]
    NotActive,
    #[display(fmt = "Blocked")]
    Blocked,
    #[display(fmt = "AuthTimeout")]
    AuthTimeout,
    #[display(fmt = "Parsing Error")]
    ParseError,
    #[display(fmt = "Request Success but No Content Found")]
    NoContent,
    #[display(fmt = "Request Success but No Cache Found")]
    NoCache,
    #[display(fmt = "Ids From Cache")]
    IdsFromCache(Vec<u32>),
    #[display(fmt = "AWC Http Client Error")]
    HttpClient,
    #[display(fmt = "Mail Service Error")]
    MailingError,
}

impl ResponseError for ResError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ResError::InternalServerErrorExplained(e) => {
                HttpResponse::InternalServerError().json(ErrorMessage::new(500, e))
            }

            ResError::BadRequest => {
                HttpResponse::BadRequest().json(ErrorMessage::new(400, "Bad Request"))
            }

            ResError::BadRequestExplained(e) => {
                HttpResponse::BadRequest().json(ErrorMessage::new(400, e))
            }
            ResError::NoContent => {
                HttpResponse::NoContent().json(ErrorMessage::new(204, "No Content"))
            }
            ResError::UsernameTaken => {
                HttpResponse::BadRequest().json(ErrorMessage::new(400, "Username already taken"))
            }

            ResError::EmailTaken => {
                HttpResponse::BadRequest().json(ErrorMessage::new(400, "Email already registered"))
            }

            ResError::InvalidUsername => {
                HttpResponse::BadRequest().json(ErrorMessage::new(400, "Invalid Username"))
            }

            ResError::InvalidPassword => {
                HttpResponse::BadRequest().json(ErrorMessage::new(400, "Invalid Password"))
            }

            ResError::InvalidEmail => {
                HttpResponse::BadRequest().json(ErrorMessage::new(400, "Invalid Email"))
            }

            ResError::WrongPwd => {
                HttpResponse::Forbidden().json(ErrorMessage::new(403, "Password is wrong"))
            }

            ResError::Unauthorized => {
                HttpResponse::Forbidden().json(ErrorMessage::new(403, "Unauthorized"))
            }

            ResError::AuthTimeout => {
                HttpResponse::Forbidden().json(ErrorMessage::new(403, "Authentication Timeout"))
            }

            ResError::ParseError => {
                HttpResponse::InternalServerError().json(ErrorMessage::new(500, "Parsing error"))
            }

            ResError::NotActive => {
                HttpResponse::Forbidden().json(ErrorMessage::new(403, "User is not activated yet"))
            }

            ResError::Blocked => {
                HttpResponse::Forbidden().json(ErrorMessage::new(403, "User is blocked"))
            }

            _ => HttpResponse::InternalServerError()
                .json(ErrorMessage::new(500, "Internal Server Error")),
        }
    }
}

impl From<PostgresPoolError> for ResError {
    fn from(e: PostgresPoolError) -> Self {
        match e {
            PostgresPoolError::Inner(e) => e.into(),
            PostgresPoolError::TimeOut => {
                ResError::PostgresExplained("Database request timeout".into())
            }
        }
    }
}

impl From<RedisPoolError> for ResError {
    fn from(e: RedisPoolError) -> Self {
        match e {
            RedisPoolError::Inner(e) => e.into(),
            RedisPoolError::TimeOut => {
                ResError::RedisErrorExplained("Redis request timeout".into())
            }
        }
    }
}

impl From<tokio_postgres::Error> for ResError {
    fn from(e: tokio_postgres::Error) -> ResError {
        ResError::BadRequestExplained(format!("{}", e))
    }
}

impl From<redis::RedisError> for ResError {
    fn from(e: redis::RedisError) -> ResError {
        ResError::RedisErrorExplained(format!("{}", e))
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

impl<E: Debug + Display> From<BlockingError<E>> for ResError {
    fn from(e: BlockingError<E>) -> ResError {
        ResError::InternalServerErrorExplained(format!("{}", e))
    }
}

impl From<serde_json::Error> for ResError {
    fn from(e: serde_json::Error) -> ResError {
        ResError::InternalServerErrorExplained(format!("{}", e))
    }
}

impl From<std::num::ParseIntError> for ResError {
    fn from(e: std::num::ParseIntError) -> ResError {
        ResError::InternalServerErrorExplained(format!("{}", e))
    }
}

impl From<chrono::format::ParseError> for ResError {
    fn from(e: chrono::format::ParseError) -> ResError {
        ResError::InternalServerErrorExplained(format!("{}", e))
    }
}

impl From<lettre_email::error::Error> for ResError {
    fn from(_e: lettre_email::error::Error) -> ResError {
        ResError::InternalServerError
    }
}

//ToDo: handle smtp error
impl From<lettre::smtp::error::Error> for ResError {
    fn from(_e: lettre::smtp::error::Error) -> ResError {
        ResError::MailingError
    }
}

//ToDo: handle psn error.
impl From<PSNError> for ResError {
    fn from(e: PSNError) -> ResError {
        ResError::InternalServerErrorExplained(format!("{}", e))
    }
}

#[derive(Serialize, Debug, Eq, PartialEq, Hash)]
pub struct DatabaseErrorMessage {
    category: Option<String>,
    description: String,
}

#[derive(Serialize)]
struct ErrorMessage<'a> {
    status: u16,
    error: &'a str,
}

impl<'a> ErrorMessage<'a> {
    fn new(status: u16, error: &'a str) -> Self {
        Self { status, error }
    }
}

// report error will be sent to users by sms/email/message
#[derive(Debug, Display, Hash, Eq, PartialEq)]
pub enum RepError {
    Ignore,
    Database,
    Redis,
    Mailer,
    HttpClient,
}

impl From<ResError> for RepError {
    fn from(e: ResError) -> RepError {
        match e {
            ResError::PostgresError => RepError::Database,
            ResError::RedisError => RepError::Redis,
            ResError::HttpClient => RepError::HttpClient,
            ResError::MailingError => RepError::Mailer,
            _ => RepError::Ignore,
        }
    }
}
