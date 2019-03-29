use actix_web::{AsyncResponder, FutureResponse, HttpResponse, ResponseError, State, Json};
use futures::{Future, future::result};

use crate::app::AppState;
use crate::model::response::Response;
use crate::model::{category::*, user::*, topic::*};
use crate::model::errors::ServiceError;
use crate::model::topic::TopicQuery;

pub fn admin_modify_category((category_request, state): (Json<CategoryRequest>, State<AppState>))
                             -> FutureResponse<HttpResponse> {
    match category_request.modify_type {
        Some(request_type) => if request_type > 2 {
            Box::new(result(Ok(ServiceError::NotFound.error_response())))
        } else {
            state.db
                .send(CategoryQuery::ModifyCategory(CategoryRequest {
                    categories: None,
                    modify_type: category_request.modify_type.clone(),
                    category_id: category_request.category_id.clone(),
                    category_data: category_request.category_data.clone(),
                    page: None,
                }))
                .from_err()
                .and_then(|db_response| match db_response {
                    Ok(_) => Ok(Response::Modified.response()),
                    Err(service_error) => Ok(service_error.error_response())
                })
                .responder()
        },
        None => Box::new(result(Ok(ServiceError::InternalServerError.error_response())))
    }
}

pub fn admin_update_user((user_update_request, state): (Json<UserUpdateRequest>, State<AppState>))
                         -> FutureResponse<HttpResponse> {
    state.db
        .send(UserQuery::UpdateUser(UserUpdateRequest {
            id: user_update_request.id.clone(),
            username: None,
            password: None,
            email: None,
            avatar_url: None,
            signature: None,
            is_admin: user_update_request.is_admin.clone(),
            blocked: user_update_request.blocked.clone(),
        }))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(_) => Ok(Response::Modified.response()),
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}

pub fn admin_update_topic((topic_update_request, state): (Json<TopicUpdateRequest>, State<AppState>))
                          -> FutureResponse<HttpResponse> {
    state.db
        .send(TopicQuery::UpdateTopic(topic_update_request.clone()))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(_) => Ok(Response::Modified.response()),
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}