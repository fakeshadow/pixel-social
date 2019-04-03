use actix_web::{web, Error, HttpResponse, ResponseError};
use futures::IntoFuture;

use crate::handler::auth::UserJwt;
use crate::handler::topic::topic_handler;
use crate::model::common::{GlobalGuard, PostgresPool, QueryOption, RedisPool, ResponseMessage};
use crate::model::errors::ServiceError;
use crate::model::topic::*;

pub fn test_lock(
    global_var: web::Data<GlobalGuard>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item = HttpResponse, Error = ServiceError> {
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
        cache_pool: None,
        global_var: Some(&global_var),
    };

    match_query_result(topic_handler(topic_query, opt))
}

fn match_query_result(
    result: Result<TopicQueryResult, ServiceError>,
) -> Result<HttpResponse, ServiceError> {
    match result {
        Ok(query_result) => match query_result {
            TopicQueryResult::AddedTopic => {
                Ok(HttpResponse::Ok().json(ResponseMessage::new("Add Topic Success")))
            }
            TopicQueryResult::GotTopicSlim(topic) => Ok(HttpResponse::Ok().json(topic)),
        },
        Err(e) => Err(e),
    }
}
