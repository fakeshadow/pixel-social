use actix_web::web;
use diesel::prelude::*;

use crate::model::{
    errors::ServiceError,
    user::{User, AuthRequest, AuthResponse, UserQuery, UserQueryResult, UserUpdateRequest},
    common::{GlobalGuard, PostgresPool, QueryOption},
};
use crate::schema::users;
use crate::util::{hash, jwt};

type QueryResult = Result<UserQueryResult, ServiceError>;

impl<'a> UserQuery<'a> {
    pub fn handle_query(self, opt: &QueryOption) -> QueryResult {
        let conn: &PgConnection = &opt.db_pool.unwrap().get().unwrap();
        match self {
            UserQuery::GetMe(my_id) => get_me(&my_id, &conn),
            UserQuery::GetUser(other_username) => get_user(&other_username, &conn),
            UserQuery::Login(login_request) => login_user(&login_request, &conn),
            UserQuery::UpdateUser(user_update_request) => update_user(&user_update_request, &conn),
            UserQuery::Register(register_request) => register_user(&register_request, &opt.global_var, &conn)
        }
    }
}

fn get_me(my_id: &u32, conn: &PgConnection) -> QueryResult {
    let user: User = users::table.find(&my_id).first::<User>(conn)?;
    Ok(UserQueryResult::GotUser(user))
}

fn get_user(other_username: &str, conn: &PgConnection) -> QueryResult {
    let user = users::table
        .filter(users::username.eq(&other_username))
        .first::<User>(conn)?;
    Ok(UserQueryResult::GotPublicUser(user.into()))
}

fn login_user(login_request: &AuthRequest, conn: &PgConnection) -> QueryResult {
    let _username = login_request.username;
    let _password = login_request.password;

    let exist_user = users::table
        .filter(users::username.eq(&_username))
        .first::<User>(conn)?;

    if exist_user.blocked { return Err(ServiceError::Unauthorized); }
    hash::verify_password(&_password, &exist_user.hashed_password)?;

    let token = jwt::JwtPayLoad::new(exist_user.id, exist_user.is_admin).sign()?;
    Ok(UserQueryResult::LoggedIn(AuthResponse {
        token,
        user_data: exist_user.into(),
    }))
}

fn update_user(user_update_request: &UserUpdateRequest, conn: &PgConnection) -> QueryResult {
    let user_self_id = user_update_request.id;

    let user_old_filter = users::table.filter(users::id.eq(&user_self_id));
    let updated_user = diesel::update(user_old_filter).set(user_update_request).get_result(conn)?;

    Ok(UserQueryResult::GotUser(updated_user))
}

fn register_user(request: &AuthRequest, global_var: &Option<&web::Data<GlobalGuard>>, conn: &PgConnection) -> QueryResult {
    match users::table
        .select((users::username, users::email))
        .filter(users::username.eq(&request.username))
        .or_filter(users::email.eq(&request.email.ok_or(ServiceError::BadRequestGeneral)?))
        .load::<(String, String)>(conn)?.pop() {
        Some((exist_username, _)) => {
            if exist_username == request.username {
                Err(ServiceError::UsernameTaken)
            } else {
                Err(ServiceError::EmailTaken)
            }
        }
        None => {
            let password_hash: String = hash::hash_password(request.password)?;
            let id: u32 = global_var.unwrap().lock()
                // ToDo: In case mutex guard failed change back to increment global vars directly.
                .map(|mut guarded_global_var| guarded_global_var.next_uid())
                .map_err(|_| ServiceError::InternalServerError)?;

            diesel::insert_into(users::table)
                .values(&request.make_user(&id, &password_hash))
                .execute(conn)?;
            Ok(UserQueryResult::Registered)
        }
    }
}