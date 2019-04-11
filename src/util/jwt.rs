use std::env;

use chrono::{Duration, Local};
use jsonwebtoken::{decode, encode, Header, Validation};

use crate::model::errors::ServiceError;

#[derive(Serialize, Deserialize)]
pub struct JwtPayLoad {
    pub iat: i64,
    pub exp: i64,
    pub user_id: u32,
    pub is_admin: u32,
}

impl JwtPayLoad {
    pub fn new(user_id: u32, is_admin: u32) -> Self {
        JwtPayLoad {
            iat: Local::now().timestamp(),
            exp: (Local::now() + Duration::days(30)).timestamp(),
            user_id,
            is_admin,
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
