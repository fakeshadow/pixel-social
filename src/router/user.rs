use actix_web::{web, HttpResponse};
use futures::IntoFuture;

use crate::handler::auth::UserJwt;
use crate::model::{
    errors::ServiceError,
    user::{UserQuery, AuthJson, UserUpdateJson},
    common::{PostgresPool, QueryOption, RedisPool, ResponseMessage, Validator, GlobalGuard},
};
use crate::util::validation::validate_username;

pub fn get_user(
    user_jwt: UserJwt,
    username_path: web::Path<String>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let username = username_path.as_str();

    if !validate_username(&username) { return Err(ServiceError::InvalidUsername); }

    let opt = QueryOption::new(Some(&db_pool), None, None);

    let user_query = if username == "me" {
        UserQuery::GetMe(&user_jwt.user_id)
    } else {
        UserQuery::GetUser(&username)
    };

    Ok(user_query.handle_query(&opt)?.to_response())
}

pub fn login_user(
    json: web::Json<AuthJson>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    json.check_login()?;
    let opt = QueryOption::new(Some(&db_pool), None, None);
    Ok(UserQuery::Login(json.to_request()).handle_query(&opt)?.to_response())
}

pub fn update_user(
    user_jwt: UserJwt,
    json: web::Json<UserUpdateJson>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    if let Some(_) = json.username { json.check_username()? }
    let opt = QueryOption::new(Some(&db_pool), None, None);
    Ok(UserQuery::UpdateUser(json.to_request(&user_jwt.user_id)).handle_query(&opt)?.to_response())
}

pub fn register_user(
    global_var: web::Data<GlobalGuard>,
    json: web::Json<AuthJson>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    json.check_register()?;
    let opt = QueryOption::new(Some(&db_pool), None, Some(&global_var));
    Ok(UserQuery::Register(json.to_request()).handle_query(&opt)?.to_response())
}
