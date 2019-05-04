use futures::{IntoFuture, Future, future::result as ftr};

use actix_web::{web::{Data, Json, Path}, HttpResponse};

use crate::model::{
    errors::ServiceError,
    topic::{TopicRequest, TopicQuery},
    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool},
};
use crate::handler::{auth::UserJwt, cache::{handle_cache_query, CacheQuery}};

pub fn add_topic(jwt: UserJwt, req: Json<TopicRequest>, global: Data<GlobalGuard>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                 -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    TopicQuery::AddTopic(req.into_inner().attach_user_id(Some(jwt.user_id)))
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), Some(&global)))
        .into_future()
}

pub fn get_topic(path: Path<(u32, i64)>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                 -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let (topic_id, page) = path.into_inner();
    handle_cache_query(CacheQuery::GetTopic(&topic_id, &page), &cache).into_future()
        .then(move |res| match res {
            Ok(res) => ftr(Ok(res)),
            Err(_) => TopicQuery::GetTopic(topic_id, page)
                .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
                .into_future()
        })
}

pub fn update_topic(jwt: UserJwt, req: Json<TopicRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                    -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    TopicQuery::UpdateTopic(req.into_inner().attach_user_id(Some(jwt.user_id)))
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
        .into_future()
}
