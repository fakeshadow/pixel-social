use actix_web::{HttpResponse, web::{Data, Json, Path}};
use futures::{Future, future::result as ftr, IntoFuture};

use crate::handler::{auth::UserJwt, cache::handle_cache_query};
use crate::model::{
    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool},
    errors::ServiceError,
    user::{AuthRequest, UserUpdateRequest},
};

pub fn get_user(jwt: UserJwt, id: Path<u32>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    use crate::model::{user::IdToQuery, cache::IdToUserQuery};
    handle_cache_query(id.into_query_cache(jwt.user_id), &cache)
        .into_future()
        .then(move |res| match res {
            Ok(res) => ftr(Ok(res)),
            Err(_) => id.into_query(jwt.user_id)
                .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
                .into_future()
        })
}

pub fn login_user(req: Json<AuthRequest>, db: Data<PostgresPool>)
                  -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    req.to_login_query()
        .handle_query(&QueryOption::new(Some(&db), None, None))
        .into_future()
}

pub fn update_user(jwt: UserJwt, mut req: Json<UserUpdateRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                   -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    req.attach_id(Some(jwt.user_id))
        .to_query()
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
        .into_future()
}

pub fn register_user(global: Data<GlobalGuard>, req: Json<AuthRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                     -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    req.to_register_query()
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), Some(&global)))
        .into_future()
}
