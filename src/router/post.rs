use actix_web::{web, HttpResponse};
use futures::IntoFuture;

use crate::model::post::*;
use crate::model::response::Response;
use crate::handler::auth::UserJwt;

use crate::model::types::*;
use crate::handler::post::post_handler;
use crate::model::errors::ServiceError;

pub fn add_post(
    user_jwt: UserJwt,
    post_request: web::Json<PostRequest>,
    db: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {

    let post_query = PostQuery::AddPost(NewPost {
        user_id: user_jwt.user_id,
        post_id: post_request.post_id.clone(),
        topic_id: post_request.topic_id.clone(),
        post_content: post_request.post_content.clone(),
    });

    match_query_result(post_handler(post_query, db))
}

pub fn get_post(
    _: UserJwt,
    post_id: web::Path<i32>,
    db: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {

    let post_id = post_id.into_inner();
    let post_query = PostQuery::GetPost(post_id);

    match_query_result(post_handler(post_query, db))
}

pub fn update_post(
    user_jwt: UserJwt,
    post_request: web::Json<PostRequest>,
    db: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {

    let post_query = PostQuery::EditPost(NewPost {
        user_id: user_jwt.user_id,
        post_id: post_request.post_id.clone(),
        topic_id: post_request.topic_id.clone(),
        post_content: post_request.post_content.clone(),
    });

    match_query_result(post_handler(post_query, db))
}

fn match_query_result(result: Result<PostQueryResult, ServiceError>) -> Result<HttpResponse, ServiceError> {
    match result {
        Ok(query_result) => {
            match query_result {
                PostQueryResult::AddedPost => Ok(Response::Post.response()),
                PostQueryResult::GotPost(post) => Ok(HttpResponse::Ok().json(post)),
            }
        }
        Err(e) => Err(e)
    }
}