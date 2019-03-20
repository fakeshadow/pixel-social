use actix_web::{AsyncResponder, FutureResponse, HttpResponse, ResponseError, State, Json, Path};
use futures::Future;

use crate::app::AppState;
use crate::model::post::*;
use crate::model::response::Response;
use crate::handler::auth::UserJwt;

pub fn add_post((post_request, state, user_jwt): (Json<PostRequest>, State<AppState>, UserJwt))
                -> FutureResponse<HttpResponse> {
    let to_pid = match post_request.to_pid {
        Some(to_pid) => to_pid,
        None => -1
    };

    state.db
        .send(PostQuery::AddPost(NewPost {
            user_id: user_jwt.user_id.clone(),
            to_pid,
            to_tid: post_request.to_tid.clone(),
            post_content: post_request.post_content.clone(),
        }))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(_) => Ok(Response::Post(true).response()),
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
            Ok(query_result) => {
                match query_result.to_post_data() {
                    Some(post_data) => Ok(Response::GetPost(post_data).response()),
                    None => Ok(Response::ToError(true).response())
                }
            }
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}


