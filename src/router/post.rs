use actix_web::{AsyncResponder, FutureResponse, HttpResponse, ResponseError, State, Json, Path};
use futures::Future;

use crate::app::AppState;
use crate::model::post::*;
use crate::model::response::Response;
use crate::handler::auth::UserJwt;

pub fn add_post((post_request, state, user_jwt): (Json<PostRequest>, State<AppState>, UserJwt))
                -> FutureResponse<HttpResponse> {
    state.db
        .send(PostQuery::AddPost(NewPost {
            user_id: user_jwt.user_id.clone(),
            post_id: post_request.post_id.clone(),
            topic_id: post_request.topic_id.clone(),
            post_content: post_request.post_content.clone(),
        }))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(query_result) => Ok(match_query_result(query_result)),
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}

pub fn get_post((post_id, state, _): (Path<i32>, State<AppState>, UserJwt))
                -> FutureResponse<HttpResponse> {
    let post_id = post_id.into_inner();

    state.db
        .send(PostQuery::GetPost(post_id))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(query_result) => Ok(match_query_result(query_result)),
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}

pub fn update_post((post_request, state, user_jwt): (Json<PostRequest>, State<AppState>, UserJwt))
                   -> FutureResponse<HttpResponse> {
    state.db
        .send(PostQuery::EditPost(NewPost {
            user_id: user_jwt.user_id.clone(),
            post_id: post_request.post_id.clone(),
            topic_id: post_request.topic_id.clone(),
            post_content: post_request.post_content.clone(),
        }))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(query_result) => Ok(match_query_result(query_result)),
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}

fn match_query_result(result: PostQueryResult) -> HttpResponse{
    match result {
        PostQueryResult::AddedPost => Response::Post.response(),
        PostQueryResult::GotPost(post) => HttpResponse::Ok().json(post),
    }
}