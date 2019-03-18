use frank_jwt::{Algorithm, encode, Error};
use chrono::{NaiveDateTime, Local};

#[derive(Debug, Serialize)]
pub struct JwtPayLoad<'a> {
    pub iat: NaiveDateTime,
    pub uid: &'a u32,
}

impl<'a> JwtPayLoad<'a> {
    pub fn new(uid: &'a u32) -> Self {
        JwtPayLoad {
            iat: Local::now().naive_local(),
            uid,
        }
    }
    pub fn sign(&self) -> Result<String, Error> {
        let secret = String::from("123456");
        encode(json!({}), &secret, &json!(self), Algorithm::HS256)
    }
}