use actix_web::{web, Error, HttpResponse, ResponseError};
use futures::{IntoFuture, Future};

use crate::handler::{auth::UserJwt, cache::*, category::*, topic::*};
use crate::model::{
    cache::*,
    category::*,
    topic::*,
    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool, ResponseMessage},
    errors::ServiceError,
};

pub fn test_global_var(
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

// async test of db query. not good result for now
pub fn get_category_async(
    db_pool: web::Data<PostgresPool>,
) -> impl Future<Item=HttpResponse, Error=ServiceError> {

    let categories = vec![1];
    let page = 1i64;
    let category_request = CategoryRequestTest {
        categories,
        page,
    };

    let query = CategoryQueryTest::GetCategory(category_request);
    use crate::handler::category::category_handler_test;

    category_handler_test(query, db_pool.clone())
        .from_err()
        .and_then(|result| match result {
            CategoryQueryResult::GotCategories(categories) => {
                Ok(HttpResponse::Ok().json(categories))
            }
            CategoryQueryResult::GotTopics(topics) => {
                Ok(HttpResponse::Ok().json(topics))
            }
            CategoryQueryResult::UpdatedCategory => {
                Ok(HttpResponse::Ok().json(ResponseMessage::new("Modify Success")))
            }
        })
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
