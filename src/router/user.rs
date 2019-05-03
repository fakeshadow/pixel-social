use futures::{IntoFuture, Future, future::result as ftr};
use actix_web::{web, HttpResponse};

use crate::model::{
    errors::ServiceError,
    user::{UserQuery, AuthRequest, UserUpdateJson},
    common::{PostgresPool, QueryOption, RedisPool, GlobalGuard},
};
use crate::handler::{auth::UserJwt, cache::{handle_cache_query, CacheQuery}};

pub fn get_user(
    user_jwt: UserJwt,
    id: web::Path<u32>,
    db_pool: web::Data<PostgresPool>,
    cache_pool: web::Data<RedisPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let id = id.into_inner();
    let (cache_query, user_query) = if id == user_jwt.user_id {
        (CacheQuery::GetMe(user_jwt.user_id), UserQuery::GetMe(user_jwt.user_id))
    } else {
        (CacheQuery::GetUser(id), UserQuery::GetUser(id))
    };
    handle_cache_query(cache_query, &cache_pool).into_future()
        .then(move |res| match res {
            Ok(res) => ftr(Ok(res)),
            Err(_) => user_query.handle_query(&QueryOption::new(Some(&db_pool), Some(&cache_pool), None)).into_future()
        })
}

pub fn login_user(
    req: web::Json<AuthRequest>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);
    UserQuery::Login(&req.into_inner()).handle_query(&opt).into_future()
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
    req: web::Json<AuthRequest>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, Some(&global_var));
    UserQuery::Register(&req.into_inner()).handle_query(&opt).into_future()
}
