use actix_web::{web::{Data, Json, Path}, HttpResponse};
use futures::IntoFuture;

use crate::model::{
    errors::ServiceError,
    post::{PostQuery, PostRequest},
    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool},
};
use crate::handler::auth::UserJwt;

pub fn add_post(jwt: UserJwt, req: Json<PostRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>, global: Data<GlobalGuard>)
                -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    PostQuery::AddPost(&mut req.into_inner().attach_user_id(Some(jwt.user_id)))
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), Some(&global)))
        .into_future()
}

pub fn get_post(_: UserJwt, post_path: Path<u32>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    PostQuery::GetPost(post_path.as_ref())
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
        .into_future()
}

pub fn update_post(jwt: UserJwt, req: Json<PostRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                   -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    PostQuery::UpdatePost(&req.into_inner().attach_user_id(Some(jwt.user_id)))
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
        .into_future()
}