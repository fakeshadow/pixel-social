use actix_web::{web, Error, HttpResponse, ResponseError};
use futures::Future;

use crate::model::{topic::*, response::Response};
use crate::handler::auth::UserJwt;

use crate::model::types::*;
use crate::handler::topic::topic_handler;

pub fn add_topic(
    user_jwt: UserJwt,
    topic_request: web::Json<TopicRequest>,
    db: web::Data<PostgresPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {

    let topic_query = TopicQuery::AddTopic(NewTopic {
        user_id: user_jwt.user_id.clone(),
        category_id: topic_request.category_id.clone(),
        thumbnail: topic_request.thumbnail.clone(),
        title: topic_request.title.clone(),
        body: topic_request.body.clone(),
    });

    web::block(move || topic_handler(topic_query, db))
        .then(|db_response|
            match db_response {
                Ok(query_result) => Ok(match_query_result(query_result)),
                Err(service_error) => Ok(service_error.error_response())
            })
}

pub fn get_topic(
    _: UserJwt,
    query_path: web::Path<(u32, u32)>,
    db: web::Data<PostgresPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {

    let (topic_id, page) = query_path.into_inner();
    let topic_query = TopicQuery::GetTopic(topic_id as i32, page as i64);

    web::block(move || topic_handler(topic_query, db))
        .then(|db_response|
            match db_response {
                Ok(query_result) => Ok(match_query_result(query_result)),
                Err(service_error) => Ok(service_error.error_response())
            })
}

pub fn update_topic(
    user_jwt: UserJwt,
    topic_update_request: web::Json<TopicUpdateRequest>,
    db: web::Data<PostgresPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let topic_query = TopicQuery::UpdateTopic(TopicUpdateRequest {
        id: topic_update_request.id.clone(),
        user_id: Some(user_jwt.user_id),
        category_id: None,
        title: topic_update_request.title.clone(),
        body: topic_update_request.body.clone(),
        thumbnail: topic_update_request.thumbnail.clone(),
        last_reply_time: None,
        is_locked: None,
        is_admin: None,
    });

    web::block(move || topic_handler(topic_query, db))
        .then(|db_response|
            match db_response {
                Ok(query_result) => Ok(match_query_result(query_result)),
                Err(service_error) => Ok(service_error.error_response())
            })
}

fn match_query_result(result: TopicQueryResult) -> HttpResponse {
    match result {
        TopicQueryResult::AddedTopic => Response::Topic.response(),
        TopicQueryResult::GotTopicSlim(topic) => HttpResponse::Ok().json(topic),
    }
}