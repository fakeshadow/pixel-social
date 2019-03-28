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
            category_id: topic_request.category_id.clone(),
            thumbnail: topic_request.thumbnail.clone(),
            title: topic_request.title.clone(),
            body: topic_request.body.clone(),
        }))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(query_result) =>  Ok(match_query_result(query_result)),
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}

pub fn get_topic((query_path, state): (Path<(u32, u32)>, State<AppState>))
                 -> FutureResponse<HttpResponse> {
    let (topic_id, page) = query_path.into_inner();
    state.db
        .send(TopicQuery::GetTopic(topic_id as i32, page as i64))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(query_result) =>  {
                Ok(match_query_result(query_result))
            },
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}

pub fn update_topic((topic_update_request, state, user_jwt): (Json<TopicUpdateRequest>, State<AppState>, UserJwt))
                    -> FutureResponse<HttpResponse> {
    state.db
        .send(TopicQuery::UpdateTopic(TopicUpdateRequest {
            id: topic_update_request.id.clone(),
            user_id: Some(user_jwt.user_id),
            category_id: None,
            title: topic_update_request.title.clone(),
            body: topic_update_request.body.clone(),
            thumbnail: topic_update_request.thumbnail.clone(),
            last_reply_time: None,
            is_locked: None,
            is_admin: None
        }))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(query_result) =>  Ok(match_query_result(query_result)),
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}

fn match_query_result(result: TopicQueryResult) -> HttpResponse{
    match result {
        TopicQueryResult::AddedTopic => Response::Topic(true).response(),
        TopicQueryResult::GotTopicSlim(topic) => Response::SendData(topic).response()
    }
}