use actix_web::{web, Error, HttpResponse, ResponseError};
use futures::IntoFuture;

use crate::model::{
    errors::ServiceError,
    cache::{CacheQuery, TopicCacheRequest},
    topic::{TopicJson, TopicUpdateJson, TopicQuery},
    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool, SelfHaveField},
};
use crate::handler::{
    auth::UserJwt,
    cache::{match_cache_query_result, cache_handler},
};

pub fn add_topic(
    user_jwt: UserJwt,
    json: web::Json<TopicJson>,
    global_var: web::Data<GlobalGuard>,
    db_pool: web::Data<PostgresPool>,
    cache_pool: web::Data<RedisPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, Some(&global_var));
    Ok(TopicQuery::AddTopic(&json.to_request(&user_jwt.user_id)).handle_query(&opt)?.to_response())
}

pub fn get_topic(
    _: UserJwt,
    topic_path: web::Path<(u32, i64)>,
    db_pool: web::Data<PostgresPool>,
    cache_pool: web::Data<RedisPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let (topic_id, page) = topic_path.as_ref();
    let cache_page = *page as isize;

    let opt = QueryOption::new(Some(&db_pool), None, None);
    Ok(TopicQuery::GetTopic(&topic_id, &page).handle_query(&opt)?.to_response())
}

pub fn update_topic(
    user_jwt: UserJwt,
    json: web::Json<TopicUpdateJson>,
    db_pool: web::Data<PostgresPool>,
    cache_pool: web::Data<RedisPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);
    Ok(TopicQuery::UpdateTopic(&json.to_request(Some(&user_jwt.user_id))).handle_query(&opt)?.to_response())
}
