use actix_web::web;
use diesel::prelude::*;

use crate::model::errors::ServiceError;
use crate::model::{user::*};

use crate::schema::users;
use crate::util::hash;
use crate::util::jwt;

use crate::model::types::*;

pub fn user_handler(user_query: UserQuery, db_pool: web::Data<PostgresPool>) -> Result<UserQueryResult, ServiceError> {

    let conn: &PgConnection = &db_pool.get().unwrap();
    let select_user_column = (
        users::id,
        users::username,
        users::email,
        users::avatar_url,
        users::signature,
        users::created_at,
        users::updated_at,
    );

    match user_query {

        UserQuery::GetMe(my_id) => {
            let user: Option<User> = users::table.filter(users::id.eq(&my_id)).load::<User>(conn)?.pop();
            match user {
                Some(user_data) => Ok(UserQueryResult::GotUser(user_data)),
                None => Err(ServiceError::NotFound)
            }
        }

        UserQuery::GetUser(other_username) => {
            let user: Option<SlimUser> = users::table
                .filter(users::username.eq(&other_username))
                .select(select_user_column)
                .load::<SlimUser>(conn)?.pop();
            match user {
                Some(user_data) => Ok(UserQueryResult::GotSlimUser(user_data)),
                None => Err(ServiceError::NotFound)
            }
        }

        UserQuery::Login(login_request) => {
            let _username = login_request.username.unwrap_or("".to_string());
            let _password = login_request.password.unwrap_or("".to_string());

            let exist_user: Option<User> = users::table
                .filter(users::username.eq(&_username))
                .load::<User>(conn)?.pop();
            match exist_user {
                Some(user) => {
                    if user.blocked == true {
                        return Err(ServiceError::Unauthorized);
                    }
                    match hash::verify_password(&_password, &user.hashed_password) {
                        Ok(_) => {
                            let token = match jwt::JwtPayLoad::new(user.id).sign() {
                                Ok(jwt_token) => jwt_token,
                                Err(service_error) => return Err(service_error)
                            };
                            Ok(UserQueryResult::LoggedIn(AuthResponse {
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

        UserQuery::UpdateUser(update_request) => {
            let user_id = update_request.id.unwrap_or(-1);

            let user_old = users::table.find(&user_id).first::<User>(conn)?;

            match update_request.update_user_data(user_old) {
                Ok(user_new) => {
                    let updated_user =
                        diesel::update(
                            users::table.filter(users::id.eq(&user_id)))
                            .set((
                                users::username.eq(&user_new.username),
                                users::avatar_url.eq(&user_new.avatar_url),
                                users::signature.eq(&user_new.signature)))
                            .get_result(conn)?;
                    Ok(UserQueryResult::GotUser(updated_user))
                }
                Err(_) => Err(ServiceError::InternalServerError)
            }
        }

        UserQuery::Register(register_request) => {
            let _username = register_request.username.unwrap();
            let _password = register_request.password.unwrap();
            let _email = register_request.email.unwrap();

            let exist_user: Vec<(String, String)> = users::table
                .select((users::username, users::email))
                .filter(users::username.eq(&_username))
                .or_filter(users::email.eq(&_email))
                .load(conn)?;

            if exist_user.len() > 0 {
                let (exist_username, _) = &exist_user[0];
                if exist_username == &_username {
                    Err(ServiceError::UsernameTaken)
                } else {
                    Err(ServiceError::EmailTaken)
                }
            } else {
                let password_hash: String = hash::hash_password(&_password)?;
                let user = User::new(&_username, &_email, &password_hash);

                diesel::insert_into(users::table)
                    .values(&user)
                    .execute(conn)?;
                Ok(UserQueryResult::Registered)
            }
        }
    }
}
