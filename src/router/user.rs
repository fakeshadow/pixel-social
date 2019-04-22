use actix_web::{web, HttpResponse};
use futures::IntoFuture;

use crate::handler::auth::UserJwt
;
use crate::model::{
    errors::ServiceError,
    user::{UserQuery, AuthJson, UserUpdateJson, AuthRequest, UserUpdateRequest},
    common::{PostgresPool, QueryOption, RedisPool, ResponseMessage, Validator, GlobalGuard},
};
use crate::util::validation::validate_username;

pub fn get_user(
    user_jwt: UserJwt,
    username_path: web::Path<String>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let username = username_path.as_str();

    if !validate_username(&username) {
        return Err(ServiceError::UsernameShort);
    }

    let opt = QueryOption::new(Some(&db_pool), None, None);

    let user_query = if username == "me" {
        UserQuery::GetMe(&user_jwt.user_id)
    } else {
        UserQuery::GetUser(&username)
    };

    Ok(user_query.handle_query(&opt)?.to_response())
}

pub fn login_user(
    login_request: web::Json<AuthJson>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    if !login_request.check_login() {
        return Err(ServiceError::BadRequestGeneral);
    }
    let username = login_request.get_username();
    let password = login_request.get_password();

    let opt = QueryOption::new(Some(&db_pool), None, None);

    Ok(UserQuery::Login(AuthRequest {
        username,
        password,
        email: "",
    }).handle_query(&opt)?.to_response())
}

pub fn update_user(
    user_jwt: UserJwt,
    update_request: web::Json<UserUpdateJson>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    if let Some(_) = update_request.username {
        if !update_request.check_username() {
            return Err(ServiceError::UsernameShort);
        }
    }

    let opt = QueryOption::new(Some(&db_pool), None, None);
    Ok(UserQuery::UpdateUser(UserUpdateRequest {
        id: &user_jwt.user_id,
        username: update_request.username.as_ref().map(String::as_str),
        avatar_url: update_request.avatar_url.as_ref().map(String::as_str),
        signature: update_request.signature.as_ref().map(String::as_str),
        is_admin: None,
        blocked: None,
    }).handle_query(&opt)?.to_response())
}

pub fn register_user(
    global_var: web::Data<GlobalGuard>,
    register_request: web::Json<AuthJson>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    if !register_request.check_register() {
        return Err(ServiceError::RegisterLimit);
    }
    let username = register_request.get_username();
    let email = register_request.get_email();
    let password = register_request.get_password();

    let opt = QueryOption::new(Some(&db_pool), None, Some(&global_var));

    Ok(UserQuery::Register(AuthRequest {
        username,
        email,
        password,
    }).handle_query(&opt)?.to_response())
}
