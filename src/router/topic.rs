use actix_web::{AsyncResponder, FutureResponse, HttpResponse, ResponseError, State, Json, Path};
use futures::Future;

use crate::app::AppState;
use crate::model::{topic::*, response::Response};
use crate::handler::auth::UserJwt;

pub fn add_topic((topic_request, state, user_jwt): (Json<TopicRequest>, State<AppState>, UserJwt))
                 -> FutureResponse<HttpResponse> {
    state.db
        .send(TopicQuery::AddTopic(NewTopic {
            user_id: user_jwt.user_id.clone(),
            title_content: topic_request.title_content.clone(),
            post_content: topic_request.post_content.clone(),
        }))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(_) => Ok(Response::Topic(true).response()),
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}

pub fn get_topic((topic_id, state, _): (Path<i32>, State<AppState>, UserJwt))
                 -> FutureResponse<HttpResponse> {
    let topic_id = topic_id.into_inner();
    state.db
        .send(TopicQuery::GetTopic(topic_id))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(query_result) => {
                match query_result.to_topic_data() {
                    Some(topic_data) => Ok(Response::GetTopic(topic_data).response()),
                    None => Ok(Response::ToError(true).response())
                }
            }
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}