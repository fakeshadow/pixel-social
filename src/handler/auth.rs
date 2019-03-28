use actix_web::{FromRequest, HttpRequest};
use chrono::Local;

use crate::util::jwt::JwtPayLoad;
use crate::model::errors::ServiceError;

pub type UserJwt = JwtPayLoad;

impl<S> FromRequest<S> for UserJwt {
    type Config = ();
    type Result = Result<UserJwt, ServiceError>;

    fn from_request(req: &HttpRequest<S>, _: &Self::Config) -> Self::Result {
        match req.headers().get("Authorization") {
            Some(token) => {
                let vec: Vec<&str> = token.to_str().unwrap_or("no token").rsplitn(2, " ").collect();
                match JwtPayLoad::decode(vec[0]) {
                    Ok(result) => {
                        if result.exp as i64 - Local::now().timestamp() < 0 {
                            return Err(ServiceError::AuthTimeout);
                        };
                        Ok(result)
                    }
                    Err(service_error) => Err(service_error)
                }
            }
            None => Err(ServiceError::Unauthorized)
        }
    }
}
