use actix_web::{HttpRequest, dev, FromRequest};
use chrono::Local;

use crate::model::errors::ServiceError;
use crate::util::jwt::JwtPayLoad;

pub type UserJwt = JwtPayLoad;

/// jwt token extractor
impl FromRequest for UserJwt {
    type Error = ServiceError;
    type Future = Result<UserJwt, ServiceError>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
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