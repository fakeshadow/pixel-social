use actix_web::{HttpResponse, web::{Data, Json, Path}};
use futures::{Future, future::result as ftr, IntoFuture};

use crate::handler::{auth::UserJwt, cache::handle_cache_query};
use crate::model::{
    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool},
    errors::ServiceError,
    post::PostRequest,
};

pub fn add_post(jwt: UserJwt, mut req: Json<PostRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>, global: Data<GlobalGuard>)
                -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    req.attach_user_id(Some(jwt.user_id))
        .to_add_query()
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), Some(&global)))
        .into_future()
}

pub fn update_post(jwt: UserJwt, mut req: Json<PostRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                   -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    req.attach_user_id(Some(jwt.user_id))
        .to_update_query()
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
        .into_future()
}

pub fn get_post(_: UserJwt, id: Path<u32>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    use crate::model::{cache::IdToPostQuery, post::IdToQuery};
    handle_cache_query(id.into_query_cache(), &cache)
        .into_future()
        .then(move |res| match res {
            Ok(res) => ftr(Ok(res)),
            Err(_) => id.to_query()
                .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
                .into_future()
        })
}
