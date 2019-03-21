use actix::Handler;
use diesel::prelude::*;

use crate::model::errors::ServiceError;
use crate::model::{db::DbExecutor, user::*};

use crate::schema::users::dsl::*;
use crate::util::hash;
use crate::util::jwt;

impl Handler<UserQuery> for DbExecutor {
    type Result = Result<UserQueryResult, ServiceError>;

    fn handle(&mut self, message: UserQuery, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();
        match message {
            UserQuery::Register(register_request) => {
                let exist_user: Vec<(String, String)> = users
                    .select((username, email))
                    .filter(username.eq(&register_request.username))
                    .or_filter(email.eq(&register_request.email))
                    .load(conn)?;

                if exist_user.len() > 0 {
                    let (exist_username, _) = &exist_user[0];
                    if exist_username == &register_request.username {
                        Err(ServiceError::UsernameTaken)
                    } else {
                        Err(ServiceError::EmailTaken)
                    }
                } else {
                    let password_hash: String = hash::hash_password(&register_request.password)?;
                    let user = User::new(&register_request.username, &register_request.email, &password_hash);

                    diesel::insert_into(users)
                        .values(&user)
                        .execute(conn)?;
                    Ok(UserQueryResult::Registered)
                }
            }

            UserQuery::Login(login_request) => {
                let exist_user: Option<User> = users
                    .filter(&username.eq(&login_request.username))
                    .load::<User>(conn)?.pop();
                match exist_user {
                    Some(user) => {
                        match hash::verify_password(&login_request.password, &user.hashed_password) {
                            Ok(_) => {
                                let token = match jwt::JwtPayLoad::new(user.id).sign() {
                                    Ok(jwt_token) => jwt_token,
                                    Err(service_error) => return Err(service_error)
                                };
                                Ok(UserQueryResult::LoggedIn(LoginData {
                                    token,
                                    user_data: user.slim(),
                                }))
                            }
                            Err(service_error) => Err(service_error)
                        }
                    }
                    None => Err(ServiceError::NotFound)
                }
            }

            UserQuery::GetMe(my_id) => {
                let user: Option<User> = users.filter(&id.eq(&my_id)).load::<User>(conn)?.pop();
                match user {
                    Some(user_data) => Ok(UserQueryResult::GotUser(user_data)),
                    None => Err(ServiceError::NotFound)
                }
            }

            UserQuery::GetUser(other_username) => {
                let user: Option<User> = users.filter(&username.eq(&other_username)).load::<User>(conn)?.pop();
                match user {
                    Some(user_data) => Ok(UserQueryResult::GotUser(user_data)),
                    None => Err(ServiceError::NotFound)
                }
            }

            UserQuery::UpdateUser(update_request) => {
                let user_id = update_request.id.unwrap_or(-1);
                let user_old = users.find(&user_id).first::<User>(conn)?;
                match update_request.update_user_data(user_old) {
                    Ok(user_new) => {
                        let updated_user =
                            diesel::update(
                                users.filter(id.eq(&user_id)))
                                .set((username.eq(&user_new.username), avatar_url.eq(&user_new.avatar_url), signature.eq(&user_new.signature)))
                                .get_result(conn)?;
                        Ok(UserQueryResult::GotUser(updated_user))
                    }
                    Err(_) => Err(ServiceError::InternalServerError)
                }
            }
        }
    }
}


