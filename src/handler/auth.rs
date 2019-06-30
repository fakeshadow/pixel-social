use actix_web::{dev, FromRequest, HttpRequest};

use crate::model::{
    errors::ServiceError
};
use crate::util::jwt::JwtPayLoad;

pub type UserJwt = JwtPayLoad;

/// jwt token extractor from request
impl FromRequest for JwtPayLoad {
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
                JwtPayLoad::from(vec[0])
            }
            None => Err(ServiceError::Unauthorized)
        }
    }
}
