use actix_web::{web, Error, HttpResponse, ResponseError};
use futures::IntoFuture;

use crate::model::{topic::*};
use crate::model::common::{ResponseMessage, PostgresPool, RedisPool, QueryOption, GlobalGuard};
use crate::model::errors::ServiceError;
use crate::handler::auth::UserJwt;
use crate::handler::topic::topic_handler;

pub fn test_lock(
    global_var: web::Data<GlobalGuard>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let user_id = &1;
    let category_id = &1;
    let thumbnail = "test thumbnail";
    let title = "test title";
    let body = "test body";

    let topic_query = TopicQuery::AddTopic(NewTopicRequest {
        user_id,
        category_id,
        thumbnail,
        title,
        body,
    });

    let opt = QueryOption {
        db_pool: Some(&db_pool),
        global_var: Some(&global_var)
    };

    match_query_result(topic_handler(topic_query, opt))
}

pub fn get_topic(
    _: UserJwt,
    query_path: web::Path<(u32, i64)>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let (topic_id, page) = query_path.into_inner();
    let topic_query = TopicQuery::GetTopic(&topic_id, &page);

    let opt = QueryOption {
        db_pool: Some(&db_pool),
        global_var: None
    };

    match_query_result(topic_handler(topic_query, opt))
}

pub fn update_topic(
    user_jwt: UserJwt,
    topic_update_request: web::Json<TopicUpdateRequest>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
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

    let opt = QueryOption {
        db_pool: Some(&db_pool),
        global_var: None
    };

    match_query_result(topic_handler(topic_query, opt))
}

fn match_query_result(result: Result<TopicQueryResult, ServiceError>) -> Result<HttpResponse, ServiceError> {
    match result {
        Ok(query_result) => {
            match query_result {
                TopicQueryResult::AddedTopic => Ok(HttpResponse::Ok().json(ResponseMessage::new("Add Topic Success"))),
                TopicQueryResult::GotTopicSlim(topic) => Ok(HttpResponse::Ok().json(topic)),
            }
        }
        Err(e) => Err(e)
    }
}