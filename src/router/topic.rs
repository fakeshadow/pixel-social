use futures::{IntoFuture, Future, future::result as ftr};

use actix_web::{web::{Data, Json, Path}, HttpResponse};

use crate::model::{
    errors::ServiceError,
    topic::TopicRequest,
    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool},
};
use crate::handler::{auth::UserJwt, cache::handle_cache_query};

pub fn add_topic(jwt: UserJwt, mut req: Json<TopicRequest>, global: Data<GlobalGuard>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                 -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    req.attach_user_id(Some(jwt.user_id))
        .to_add_query()
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), Some(&global)))
        .into_future()
}

pub fn update_topic(jwt: UserJwt, mut req: Json<TopicRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                    -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    req.attach_user_id(Some(jwt.user_id))
        .to_update_query()
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
        .into_future()
}

pub fn get_topic(path: Path<(u32, i64)>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                 -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    use crate::model::{cache::IdToTopicQuery, topic::IdToQuery};
    let (id, page) = path.into_inner();
    handle_cache_query(id.to_query_cache(&page), &cache)
        .into_future()
        .then(move |res| match res {
            Ok(res) => ftr(Ok(res)),
            Err(_) => id
                .into_query(page)
                .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
                .into_future()
        })
}
