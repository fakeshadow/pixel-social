use actix_web::{web, HttpResponse};
use futures::IntoFuture;

use crate::model::{
    errors::ServiceError,
    post::{PostQuery, PostJson, PostRequest, PostUpdateJson, PostUpdateRequest, PostQueryResult},
    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool, ResponseMessage},
};
use crate::handler::{
    auth::UserJwt,
    post::post_handler
};

pub fn add_post(
    user_jwt: UserJwt,
    post_json: web::Json<PostJson>,
    db_pool: web::Data<PostgresPool>,
    global_var: web::Data<GlobalGuard>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
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
    post_path: web::Path<u32>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let post_id = post_path.as_ref();
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
    update_json: web::Json<PostUpdateJson>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let post_request = PostUpdateRequest {
        id: &update_json.id,
        user_id: Some(&user_jwt.user_id),
        topic_id: None,
        post_id: None,
        post_content: update_json.post_content.as_ref().map(String::as_str),
        is_locked: None,
    };

    let post_query = PostQuery::UpdatePost(post_request);

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
