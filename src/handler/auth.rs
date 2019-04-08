use actix_web::{HttpRequest, dev::{ServiceRequest, Payload}, web, FromRequest};
use chrono::Local;

use crate::model::errors::ServiceError;
use crate::util::jwt::JwtPayLoad;

pub type UserJwt = JwtPayLoad;

impl<S> FromRequest<S> for UserJwt {
    type Error = ServiceError;
    type Future = Result<UserJwt, ServiceError>;

    fn from_request(req: &HttpRequest, _: &mut Payload<S>) -> Self::Future {
        match req.headers().get("Authorization") {
            Some(token) => {
                let vec: Vec<&str> = token
                    .to_str()
                    .unwrap_or("no token")
                    .rsplitn(2, " ")
                    .collect();

                let jwt_payload = JwtPayLoad::decode(vec[0])?;
                if jwt_payload.exp as i64 - Local::now().timestamp() < 0 {
                    return Err(ServiceError::AuthTimeout);
                };
                Ok(jwt_payload)
            }
            None => Err(ServiceError::Unauthorized)
        }
    }
}
