use actix_web::web;
use diesel::prelude::*;

use crate::model::{
    errors::ServiceError,
    user::{User, SlimUser, AuthRequest, AuthResponse, UserQuery, UserQueryResult, UserUpdateRequest},
    common::{GlobalGuard, PostgresPool, QueryOption},
};
use crate::schema::users;
use crate::util::{hash, jwt};

impl<'a> UserQuery<'a> {
    pub fn handle_query(self, opt: &QueryOption) -> Result<UserQueryResult, ServiceError> {
        let conn: &PgConnection = &opt.db_pool.unwrap().get().unwrap();
        match self {
            UserQuery::GetMe(my_id) => get_me(&my_id, &conn),
            UserQuery::GetUser(other_username) => get_user(&other_username, &conn),
            UserQuery::Login(login_request) => login_user(&login_request, &conn),
            UserQuery::UpdateUser(user_update_request) => update_user(user_update_request, &conn),
            UserQuery::Register(register_request) => register_user(&register_request, &opt.global_var, &conn)
        }
    }
}

fn get_me(my_id: &u32, conn: &PgConnection) -> Result<UserQueryResult, ServiceError> {
    let user: User = users::table
        .find(&my_id)
        .first::<User>(conn)?;

    Ok(UserQueryResult::GotUser(user))
}

fn get_user(other_username: &str, conn: &PgConnection) -> Result<UserQueryResult, ServiceError> {
    let user = users::table
        .filter(users::username.eq(&other_username))
        .select((users::id,
                 users::username,
                 users::email,
                 users::avatar_url,
                 users::signature,
                 users::created_at,
                 users::updated_at))
        .first::<SlimUser>(conn)?;

    Ok(UserQueryResult::GotSlimUser(user))
}

fn login_user(login_request: &AuthRequest, conn: &PgConnection) -> Result<UserQueryResult, ServiceError> {
    let _username = login_request.username;
    let _password = login_request.password;

    let exist_user = users::table
        .filter(users::username.eq(&_username))
        .first::<User>(conn)?;

    if exist_user.blocked { return Err(ServiceError::Unauthorized); }

    let _check_password = hash::verify_password(&_password, &exist_user.hashed_password)?;

    let token = jwt::JwtPayLoad::new(exist_user.id, exist_user.is_admin).sign()?;

    Ok(UserQueryResult::LoggedIn(AuthResponse {
        token,
        user_data: exist_user.slim(),
    }))
}

fn update_user(user_update_request: UserUpdateRequest, conn: &PgConnection) -> Result<UserQueryResult, ServiceError> {
    let user_self_id = user_update_request.id;

    let user_old_filter = users::table.filter(users::id.eq(&user_self_id));
    let updated_user = diesel::update(user_old_filter).set(&user_update_request).get_result(conn)?;

    Ok(UserQueryResult::GotUser(updated_user))
}

fn register_user(register_request: &AuthRequest, global_var: &Option<&web::Data<GlobalGuard>>, conn: &PgConnection) -> Result<UserQueryResult, ServiceError> {
    let _username = register_request.username;
    let _password = register_request.password;
    let _email = register_request.email;

    match users::table
        .select((users::username, users::email))
        .filter(users::username.eq(&_username))
        .or_filter(users::email.eq(&_email))
        .load::<(String, String)>(conn)?.pop() {
        Some((exist_username, _)) => {
            if exist_username == _username {
                Err(ServiceError::UsernameTaken)
            } else {
                Err(ServiceError::EmailTaken)
            }
        }
        None => {
            let password_hash: String = hash::hash_password(_password)?;
            let id: u32 = global_var
                .unwrap()
                .lock()
                .map(|mut guarded_global_var| {
                    let next_uid = guarded_global_var.next_uid;
                    guarded_global_var.next_uid += 1;
                    next_uid
                })
                .map_err(|_|ServiceError::InternalServerError)?;

            let user = User::new(id, &_username, &_email, &password_hash);
            diesel::insert_into(users::table)
                .values(&user)
                .execute(conn)?;
            Ok(UserQueryResult::Registered)
        }
    }
}