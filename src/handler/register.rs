use actix::{Handler, Message};
use diesel::prelude::*;
use bcrypt::verify;
use actix_web::{FromRequest, HttpRequest, middleware::identity::RequestIdentity};

use crate::errors::ServiceError;
use crate::models::{DbExecutor, User};


#[derive(Debug, Deserialize)]
pub struct IncomingRegister {
    pub username: String,
    pub password: String,
    pub email: String,
}

#[derive(Debug)]
pub struct RegisterData {
    pub uid: u32,
    pub username: String,
    pub password: String,
    pub email: String,
}

impl Message for RegisterData {
    type Result = Result<(), ServiceError>;
}

impl Handler<RegisterData> for DbExecutor {
    type Result = Result<(), ServiceError>;

    fn handle(&mut self, msg: RegisterData, _: &mut Self::Context) -> Self::Result {
        use crate::schema::users::dsl::users;

        let conn: &PgConnection = &self.0.get().unwrap();

//        let password: String = hash_password(&msg.password)?;
        let user = User::create(msg.uid, msg.username, msg.email, msg.password);

        diesel::insert_into(users)
            .values(&user)
            .execute(conn)?;
        Ok(())
    }
}