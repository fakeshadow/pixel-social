use actix::{Handler, Message};
use diesel::prelude::*;

use crate::util::jwt;
use crate::errors::ServiceError;
use crate::model::{
    user::{User, LoggedInData, LoginData},
    db::DbExecutor,
};
use crate::util::hash::verify_password;
use crate::schema::users::dsl::*;

impl Message for LoginData {
    type Result = Result<LoggedInData, ServiceError>;
}

impl Handler<LoginData> for DbExecutor {
    type Result = Result<LoggedInData, ServiceError>;

    fn handle(&mut self, msg: LoginData, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();

        //let password: String = hash_password(&msg.password)?;

        let exist_user: Option<User> = users
            .filter(&username.eq(&msg.username))
            .load::<User>(conn)?.pop();
        match exist_user {
            Some(user) => {
                match verify_password(&msg.password, &user.password) {
                    Ok(_) => {
                        let token= match jwt::JwtPayLoad::new(&user.uid).sign() {
                            Ok(jwt_token) => jwt_token,
                            Err(_) => return Err(ServiceError::InternalServerError)
                        };
                        Ok(LoggedInData {
                            token,
                            user_data: user.slim(),
                        })
                    },
                    Err(service_error) =>Err(service_error)
                }
            }
            None => Err(ServiceError::NoUser)
        }
    }
}