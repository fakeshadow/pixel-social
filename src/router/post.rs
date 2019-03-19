use actix_web::{AsyncResponder, FutureResponse, HttpResponse, ResponseError, State, Json};
use futures::{Future,future::result};

use crate::app::AppState;
use crate::model::post::IncomingPost;
use crate::model::response::Response;
use crate::handler::auth::UserJwt;

pub fn add_post((incoming_post, state, user_jwt): (Json<IncomingPost>, State<AppState>, UserJwt))
                -> FutureResponse<HttpResponse> {
    state.db
        .send(IncomingPost {
            uid: incoming_post.uid.clone(),
            to_pid: incoming_post.to_pid.clone(),
            to_tid: incoming_post.to_tid.clone(),
            post_content: incoming_post.post_content.clone(),
        })
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(_) => Ok(Response::PostSuccess(true).response()),
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}

pub fn get_post((state, user_jwt): (State<AppState>, UserJwt)) -> FutureResponse<HttpResponse> {
    Box::new(result(Ok(HttpResponse::Ok().json(user_jwt))))

}