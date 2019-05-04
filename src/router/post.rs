use futures::{IntoFuture, Future, future::result as ftr};

use actix_web::{web::{Data, Json, Path}, HttpResponse};

use crate::model::{
    errors::ServiceError,
    post::{PostQuery, PostRequest},
    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool},
};
use crate::handler::{
    auth::UserJwt,
    cache::{handle_cache_query, CacheQuery},
};

pub fn add_post(jwt: UserJwt, req: Json<PostRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>, global: Data<GlobalGuard>)
                -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    PostQuery::AddPost(&mut req.into_inner().attach_user_id(Some(jwt.user_id)))
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), Some(&global)))
        .into_future()
}

pub fn get_post(_: UserJwt, path: Path<u32>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let id = path.into_inner();
    handle_cache_query(CacheQuery::GetPost(id), &cache).into_future()
        .then(move |res| match res {
            Ok(res) => ftr(Ok(res)),
            Err(_) => {
                PostQuery::GetPost(&id)
                    .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
                    .into_future()
            }
        })
}

pub fn update_post(jwt: UserJwt, req: Json<PostRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                   -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    PostQuery::UpdatePost(&req.into_inner().attach_user_id(Some(jwt.user_id)))
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
        .into_future()
}