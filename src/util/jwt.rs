use std::env;

use chrono::{Duration, Local};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

use crate::model::errors::ResError;

#[derive(Serialize, Deserialize, Debug)]
pub struct JwtPayLoad {
    pub exp: i64,
    pub user_id: u32,
    pub privilege: u32,
}

impl JwtPayLoad {
    pub fn new(user_id: u32, privilege: u32) -> Self {
        JwtPayLoad {
            exp: (Local::now() + Duration::days(30)).timestamp(),
            user_id,
            privilege,
        }
    }

    pub fn from(string: &str) -> Result<JwtPayLoad, ResError> {
        let decoded_key = DecodingKey::from_base64_secret(key_string().as_str())
            .expect("Fatal error when encoding JWT secret");

        let token: JwtPayLoad = decode::<JwtPayLoad>(string, &decoded_key, &Validation::default())
            .map(|data| data.claims)
            .map_err(|_| ResError::Unauthorized)?;
        if token.exp as i64 - Local::now().timestamp() < 0 {
            Err(ResError::AuthTimeout)
        } else {
            Ok(token)
        }
    }

    pub fn sign(&self) -> Result<String, ResError> {
        let encoded_key = EncodingKey::from_base64_secret(key_string().as_str())
            .expect("Fatal error when encoding JWT secret");

        encode(&Header::default(), &self, &encoded_key).map_err(|_| ResError::InternalServerError)
    }

    pub fn check_privilege(&self) -> Result<(), ResError> {
        self.check_active()?;
        self.check_blocked()?;
        Ok(())
    }

    pub fn check_active(&self) -> Result<(), ResError> {
        if self.privilege > 1 {
            Ok(())
        } else {
            Err(ResError::NotActive)
        }
    }

    pub fn check_blocked(&self) -> Result<(), ResError> {
        if self.privilege == 0 {
            Err(ResError::Blocked)
        } else {
            Ok(())
        }
    }
}

fn key_string() -> String {
    env::var("JWT_SECRET").unwrap_or_else(|_| "fallback secret".into())
}
