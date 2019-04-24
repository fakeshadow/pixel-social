use actix_web::{web, HttpResponse};
use futures::IntoFuture;

use crate::model::{
    errors::ServiceError,
    post::{PostQuery, PostJson},
    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool},
};
use crate::handler::auth::UserJwt;

pub fn add_post(
    user_jwt: UserJwt,
    json: web::Json<PostJson>,
    db_pool: web::Data<PostgresPool>,
    global_var: web::Data<GlobalGuard>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, Some(&global_var));
    Ok(PostQuery::AddPost(&mut json.to_request(Some(&user_jwt.user_id))).handle_query(&opt)?.to_response())
}

pub fn get_post(
    _: UserJwt,
    post_path: web::Path<u32>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);
    Ok(PostQuery::GetPost(post_path.as_ref()).handle_query(&opt)?.to_response())
}

pub fn update_post(
    user_jwt: UserJwt,
    json: web::Json<PostJson>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);
    Ok(PostQuery::UpdatePost(&json.to_request(Some(&user_jwt.user_id))).handle_query(&opt)?.to_response())
}