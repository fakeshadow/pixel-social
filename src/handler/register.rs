use actix::{Handler, Message};
use diesel::prelude::*;

use crate::errors::ServiceError;
use crate::model::{
    user::{User, RegisterCheck, RegisterData},
    db::DbExecutor,
};

use crate::schema::users::dsl::*;
use crate::util::hash;

impl Message for RegisterData {
    type Result = Result<(), ServiceError>;
}

impl Message for RegisterCheck {
    type Result = Result<(), ServiceError>;
}

impl Handler<RegisterData> for DbExecutor {
    type Result = Result<(), ServiceError>;

    fn handle(&mut self, msg: RegisterData, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();

        let password_hash: String = hash::hash_password(&msg.password)?;
        let user = User::create(msg.uid, msg.username, msg.email, password_hash);

        diesel::insert_into(users)
            .values(&user)
            .execute(conn)?;
        Ok(())
    }
}

impl Handler<RegisterCheck> for DbExecutor {
    type Result = Result<(), ServiceError>;

    fn handle(&mut self, msg: RegisterCheck, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();

        let exist_user: Vec<(String, String)> = users
            .select((username, email))
            .filter(username.eq(&msg.username))
            .or_filter(email.eq(&msg.email))
            .load(conn)?;

        if exist_user.len() == 0 {
            Ok(())
        } else {
            let (exist_username, _) = &exist_user[0];
            if exist_username == &msg.username {
                Err(ServiceError::UsernameTaken)
            } else {
                Err(ServiceError::EmailTaken)
            }
        }
    }
}