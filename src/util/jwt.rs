use std::env;

use chrono::{Duration, Local};
use jsonwebtoken::{decode, encode, Header, Validation};

use crate::model::errors::ServiceError;

#[derive(Serialize, Deserialize, Debug)]
pub struct JwtPayLoad {
    pub exp: i64,
    pub user_id: u32,
    pub is_admin: u32,
    pub is_blocked: bool,
    pub is_activate: bool,
}

impl JwtPayLoad {
    pub fn new(user_id: u32, is_admin: u32, is_blocked: bool, is_activate: bool) -> Self {
        JwtPayLoad {
            exp: (Local::now() + Duration::days(30)).timestamp(),
            user_id,
            is_admin,
            is_blocked,
            is_activate,
        }
    }
    pub fn from(string: &str) -> Result<JwtPayLoad, ServiceError> {
        let token: JwtPayLoad = decode::<JwtPayLoad>(string, get_secret().as_ref(), &Validation::default())
            .map(|data| data.claims.into())
            .map_err(|_| ServiceError::Unauthorized)?;
        if token.exp as i64 - Local::now().timestamp() < 0 {
            Err(ServiceError::AuthTimeout)
        } else {
            Ok(token)
        }
    }
    pub fn sign(&self) -> Result<String, ServiceError> {
        encode(&Header::default(), &self, get_secret().as_ref())
            .map_err(|_| ServiceError::InternalServerError)
    }
    pub fn check_active_block(&self) -> Result<(),ServiceError> {
        self.check_active()?;
        self.check_blocked()?;
        Ok(())
    }
    pub fn check_active(&self) -> Result<(), ServiceError> {
        if self.is_activate {
            Ok(())
        } else {
            Err(ServiceError::NOTACTIVE)
        }
    }
    pub fn check_blocked(&self) -> Result<(), ServiceError> {
        if self.is_blocked {
            Err(ServiceError::BLOCKED)
        } else {
            Ok(())
        }
    }
}

fn get_secret() -> String {
    env::var("JWT_SECRET").unwrap_or_else(|_| "fallback secret".into())
}
