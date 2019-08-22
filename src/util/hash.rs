use std::env;

use bcrypt::{hash, verify, DEFAULT_COST};

use crate::model::errors::ResError;

pub fn hash_password(password: &str) -> Result<String, ResError> {
    let hash_cost: u32 = match env::var("HASH_ROUNDS") {
        Ok(cost) => cost.parse::<u32>().unwrap_or(DEFAULT_COST),
        _ => DEFAULT_COST,
    };
    hash(password, hash_cost).map_err(|_| ResError::InternalServerError)
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<(), ResError> {
    match verify(password, password_hash) {
        Ok(valid) => {
            if valid {
                Ok(())
            } else {
                Err(ResError::WrongPwd)
            }
        }
        _ => Err(ResError::InternalServerError),
    }
}
