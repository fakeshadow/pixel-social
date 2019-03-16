use actix::{Handler, Message};
use diesel::prelude::*;
use bcrypt::verify;
use actix_web::{FromRequest, HttpRequest, middleware::identity::RequestIdentity};

use crate::errors::ServiceError;
use crate::models::{DbExecutor, User};


#[derive(Debug, Deserialize)]
pub struct LoginData {
    pub username: String,
    pub password: String,
}

impl Message for LoginData {
    type Result = Result<(), ServiceError>;
}

impl Handler<LoginData> for DbExecutor {
    type Result = Result<(), ServiceError>;

    fn handle(&mut self, msg: LoginData, _: &mut Self::Context) -> Self::Result {
//        let conn: &PgConnection = &self.0.get().unwrap();

//        let password: String = hash_password(&msg.password)?;

        Ok(())
    }
}