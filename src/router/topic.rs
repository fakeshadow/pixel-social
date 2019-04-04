use actix_web::{web, Error, HttpResponse, ResponseError};
use futures::IntoFuture;

use crate::model::{
    topic::*,
    cache::{CacheQuery, TopicCacheRequest},
    errors::ServiceError,
    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool, ResponseMessage, SelfHaveField},
};
use crate::handler::{
    auth::UserJwt,
    topic::topic_handler,
    cache::*,
};

pub fn add_topic(
    user_jwt: UserJwt,
    topic_json: web::Json<TopicJson>,
    global_var: web::Data<GlobalGuard>,
    db_pool: web::Data<PostgresPool>,
    cache_pool: web::Data<RedisPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let user_id = &user_jwt.user_id;
    let category_id = &topic_json.category_id;
    let thumbnail = &topic_json.thumbnail;
    let title = &topic_json.title;
    let body = &topic_json.body;

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

    match_query_result(topic_handler(topic_query, opt), &cache_pool)
}

pub fn get_topic(
//    _: UserJwt,
    query_path: web::Path<(u32, i64)>,
    db_pool: web::Data<PostgresPool>,
    cache_pool: web::Data<RedisPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let (topic_id, page) = query_path.into_inner();

    let cache_page = page as isize;

    let cache_query = CacheQuery::GetTopic(TopicCacheRequest {
        topic: &topic_id,
        page: &cache_page,
    });

    match match_cache_query_result(cache_handler(cache_query, &cache_pool)) {
        Ok(cache) => Ok(cache),
        Err(_) => {
            let topic_query = TopicQuery::GetTopic(&topic_id, &page);

            let opt = QueryOption {
                db_pool: Some(&db_pool),
                cache_pool: None,
                global_var: None,
            };

            match_query_result(topic_handler(topic_query, opt), &cache_pool)
        }
    }

//    let topic_query = TopicQuery::GetTopic(&topic_id, &page);
//
//    let opt = QueryOption {
//        db_pool: Some(&db_pool),
//        cache_pool: None,
//        global_var: None,
//    };
//
//    match_query_result(topic_handler(topic_query, opt))
}

pub fn update_topic(
    user_jwt: UserJwt,
    topic_update_request: web::Json<TopicUpdateRequest>,
    db_pool: web::Data<PostgresPool>,
    cache_pool: web::Data<RedisPool>,
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
        cache_pool: None,
        global_var: None,
    };

    match_query_result(topic_handler(topic_query, opt), &cache_pool)
}

fn match_query_result(
    result: Result<TopicQueryResult, ServiceError>,
    cache_pool: &web::Data<RedisPool>,
) -> Result<HttpResponse, ServiceError> {
    match result {
        Ok(query_result) => match query_result {
            TopicQueryResult::AddedTopic => {

                Ok(HttpResponse::Ok().json(ResponseMessage::new("Add Topic Success")))
            }
            TopicQueryResult::GotTopicSlim(topic_with_post) => {
                if !topic_with_post.have_post() || !topic_with_post.have_topic() {
                    cache_handler(CacheQuery::UpdateTopic(&topic_with_post), &cache_pool);
                }
                Ok(HttpResponse::Ok().json(topic_with_post))
            }
        },
        Err(e) => Err(e),
    }
}
