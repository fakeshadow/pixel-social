use actix_web::{web, HttpResponse};
use futures::IntoFuture;

use crate::handler::auth::UserJwt;
use crate::handler::post::post_handler;
use crate::model::common::{GlobalGuard, PostgresPool, QueryOption, RedisPool, ResponseMessage};
use crate::model::errors::ServiceError;
use crate::model::post::*;

pub fn add_post(
    user_jwt: UserJwt,
    post_json: web::Json<PostJson>,
    db_pool: web::Data<PostgresPool>,
    global_var: web::Data<GlobalGuard>,
) -> impl IntoFuture<Item = HttpResponse, Error = ServiceError> {
    let post_query = PostQuery::AddPost(PostRequest {
        user_id: &user_jwt.user_id,
        post_id: post_json.post_id.as_ref(),
        topic_id: &post_json.topic_id,
        post_content: &post_json.post_content,
    });

    let opt = QueryOption {
        db_pool: Some(&db_pool),
        cache_pool: None,
        global_var: Some(&global_var),
    };

    match_query_result(post_handler(post_query, opt))
}

pub fn get_post(
    _: UserJwt,
    post_id: web::Path<u32>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item = HttpResponse, Error = ServiceError> {
    let post_id = post_id.into_inner();
    let post_query = PostQuery::GetPost(post_id);

    let opt = QueryOption {
        db_pool: Some(&db_pool),
        cache_pool: None,
        global_var: None,
    };

    match_query_result(post_handler(post_query, opt))
}

pub fn update_post(
    user_jwt: UserJwt,
    update_post_json: web::Json<UpdatePostJson>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item = HttpResponse, Error = ServiceError> {
    let post_query = PostQuery::EditPost(UpdatePostRequest {
        id: &update_post_json.id,
        user_id: &user_jwt.user_id,
        post_content: &update_post_json.post_content,
    });

    let opt = QueryOption {
        db_pool: Some(&db_pool),
        cache_pool: None,
        global_var: None,
    };

    match_query_result(post_handler(post_query, opt))
}

fn match_query_result(
    result: Result<PostQueryResult, ServiceError>,
) -> Result<HttpResponse, ServiceError> {
    match result {
        Ok(query_result) => match query_result {
            PostQueryResult::AddedPost => {
                Ok(HttpResponse::Ok().json(ResponseMessage::new("Add Post Success")))
            }
            PostQueryResult::GotPost(post) => Ok(HttpResponse::Ok().json(post)),
        },
        Err(e) => Err(e),
    }
}
