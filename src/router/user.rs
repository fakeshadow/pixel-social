use futures::{IntoFuture, Future, future::result as ftr};
use actix_web::{web::{Data, Json, Path}, HttpResponse};

use crate::model::{
    errors::ServiceError,
    user::{UserQuery, AuthRequest, UserUpdateJson},
    common::{PostgresPool, QueryOption, RedisPool, GlobalGuard},
};
use crate::handler::{auth::UserJwt, cache::{handle_cache_query, CacheQuery}};


pub fn get_user(jwt: UserJwt, id: Path<u32>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let id = id.into_inner();
    let (cache_query, user_query) = if id == jwt.user_id {
        (CacheQuery::GetMe(jwt.user_id), UserQuery::GetMe(jwt.user_id))
    } else {
        (CacheQuery::GetUser(id), UserQuery::GetUser(id))
    };
    handle_cache_query(cache_query, &cache).into_future()
        .then(move |res| match res {
            Ok(res) => ftr(Ok(res)),
            Err(_) => user_query.handle_query(&QueryOption::new(Some(&db), Some(&cache), None)).into_future()
        })
}

pub fn login_user(req: Json<AuthRequest>, db: Data<PostgresPool>)
                  -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    UserQuery::Login(&req.into_inner())
        .handle_query(&QueryOption::new(Some(&db), None, None))
        .into_future()
}

pub fn update_user(jwt: UserJwt, req: Json<UserUpdateJson>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                   -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    UserQuery::UpdateUser(&req.to_request(&jwt.user_id))
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
        .into_future()
}

pub fn register_user(global: Data<GlobalGuard>, req: Json<AuthRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                     -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    UserQuery::Register(&req.into_inner())
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), Some(&global)))
        .into_future()
}
