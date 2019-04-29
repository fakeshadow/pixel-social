use futures::{IntoFuture};
use actix_web::{web, HttpResponse};

use crate::handler::auth::UserJwt;
use crate::model::{
    errors::ServiceError,
    user::{UserQuery, AuthJson, UserUpdateJson},
    common::{PostgresPool, QueryOption, RedisPool, GlobalGuard},
};

pub fn get_user(
    user_jwt: UserJwt,
    username_path: web::Path<String>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let username = username_path.as_str();

    let opt = QueryOption::new(Some(&db_pool), None, None);
    let user_query = if username == "me" {
        UserQuery::GetMe(&user_jwt.user_id)
    } else {
        UserQuery::GetUser(&username)
    };

    user_query.handle_query(&opt).into_future()
}

pub fn login_user(
    json: web::Json<AuthJson>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);
    UserQuery::Login(&json.to_request()).handle_query(&opt).into_future()
}

pub fn update_user(
    user_jwt: UserJwt,
    json: web::Json<UserUpdateJson>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);
    UserQuery::UpdateUser(&json.to_request(&user_jwt.user_id)).handle_query(&opt).into_future()
}

pub fn register_user(
    global_var: web::Data<GlobalGuard>,
    json: web::Json<AuthJson>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, Some(&global_var));
    UserQuery::Register(&json.to_request()).handle_query(&opt).into_future()
}
