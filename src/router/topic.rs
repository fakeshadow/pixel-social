use actix_web::{web, Error, HttpResponse};
use futures::IntoFuture;

use crate::model::{
    errors::ServiceError,
    topic::{TopicRequest, TopicQuery},
    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool},
};
use crate::handler::auth::UserJwt;

pub fn add_topic(
    user_jwt: UserJwt,
    json: web::Json<TopicRequest>,
    global_var: web::Data<GlobalGuard>,
    db_pool: web::Data<PostgresPool>,
    cache_pool: web::Data<RedisPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, Some(&global_var));
    TopicQuery::AddTopic(json.into_inner().attach_user_id(Some(user_jwt.user_id))).handle_query(&opt).into_future()
}

pub fn get_topic(
    _: UserJwt,
    topic_path: web::Path<(u32, i64)>,
    db_pool: web::Data<PostgresPool>,
    cache_pool: web::Data<RedisPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let (topic_id, page) = topic_path.into_inner();

    let opt = QueryOption::new(Some(&db_pool), None, None);
    TopicQuery::GetTopic(topic_id, page).handle_query(&opt).into_future()
}

pub fn update_topic(
    user_jwt: UserJwt,
    json: web::Json<TopicRequest>,
    db_pool: web::Data<PostgresPool>,
    cache_pool: web::Data<RedisPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);
    TopicQuery::UpdateTopic(json.into_inner().attach_user_id(Some(user_jwt.user_id))).handle_query(&opt).into_future()
}
