use actix_web::{web, HttpResponse};
use futures::IntoFuture;

use crate::model::{
    errors::ServiceError,
    post::{PostQuery, PostRequest},
    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool},
};
use crate::handler::auth::UserJwt;

pub fn add_post(
    user_jwt: UserJwt,
    req: web::Json<PostRequest>,
    db_pool: web::Data<PostgresPool>,
    global_var: web::Data<GlobalGuard>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, Some(&global_var));
    PostQuery::AddPost(&mut req.into_inner().attach_user_id(Some(user_jwt.user_id))).handle_query(&opt).into_future()
}

pub fn get_post(
    _: UserJwt,
    post_path: web::Path<u32>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);
    PostQuery::GetPost(post_path.as_ref()).handle_query(&opt).into_future()
}

pub fn update_post(
    user_jwt: UserJwt,
    req: web::Json<PostRequest>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);
    PostQuery::UpdatePost(&req.into_inner().attach_user_id(Some(user_jwt.user_id))).handle_query(&opt).into_future()
}