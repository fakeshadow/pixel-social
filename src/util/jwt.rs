use std::env;

use jsonwebtoken::{encode, decode, Header, Validation};
use chrono::{Local, Duration};

use crate::model::errors::ServiceError;

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtPayLoad {
    pub iat: i64,
    pub exp: i64,
    pub user_id: i32,
}

impl JwtPayLoad {
    pub fn new(user_id: i32) -> Self {
        JwtPayLoad {
            iat: Local::now().timestamp(),
            exp: (Local::now() + Duration::days(30)).timestamp(),
            user_id,
        }
    }
    pub fn decode(token: &str) -> Result<JwtPayLoad, ServiceError> {
        decode::<JwtPayLoad>(token, get_secret().as_ref(), &Validation::default())
            .map(|data| Ok(data.claims.into()))
            .map_err(|_err| ServiceError::Unauthorized)?
    }

    pub fn sign(&self) -> Result<String, ServiceError> {
        encode(&Header::default(), &self, get_secret().as_ref())
            .map_err(|_err| ServiceError::InternalServerError)
    }
}

fn get_secret() -> String {
    env::var("JWT_SECRET").unwrap_or_else(|_| "fallback secret".into())
}