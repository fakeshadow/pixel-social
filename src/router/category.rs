use actix_web::{AsyncResponder, FutureResponse, HttpResponse, ResponseError, State, Json, Path};
use futures::Future;

use crate::app::AppState;
use crate::model::{category::*, response::Response};
use crate::handler::auth::UserJwt;

pub fn get_all_categories(state: State<AppState>) -> FutureResponse<HttpResponse> {
    state.db
        .send(CategoryQuery::GetAllCategories)
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(query_result) => {
                match query_result.to_categories_data() {
                    Some(categories_data) => Ok(Response::SendData(categories_data).response()),
                    None => Ok(Response::ToError(true).response())
                }
            }
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}

pub fn get_popular((page, state): (Path<(u32)>, State<AppState>))
                   -> FutureResponse<HttpResponse> {
    let page = page.into_inner();
    state.db
        .send(CategoryQuery::GetPopular(page))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(query_result) => {
                match query_result.to_topic_data() {
                    Some(topic_data) => Ok(Response::SendData(topic_data).response()),
                    None => Ok(Response::ToError(true).response())
                }
            }
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}

pub fn get_category((category_query, state, ): (Path<(u32, u32)>, State<AppState>))
                    -> FutureResponse<HttpResponse> {
    let (category_id, page) = category_query.into_inner();
    state.db
        .send(CategoryQuery::GetCategory(CategoryRequest {
            categories: Some(vec![category_id as i32]),
            modify_type: None,
            category_id: None,
            category_data: None,
            page: Some(page as i32),
        }))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(query_result) => {
                match query_result.to_topic_data() {
                    Some(topic_data) => Ok(Response::SendData(topic_data).response()),
                    None => Ok(Response::ToError(true).response())
                }
            }
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}

pub fn get_categories((category_request, state, _): (Json<CategoryRequest>, State<AppState>, UserJwt))
                      -> FutureResponse<HttpResponse> {
    state.db
        .send(CategoryQuery::GetCategory(CategoryRequest {
            categories: category_request.categories.clone(),
            modify_type: None,
            category_id: None,
            category_data: None,
            page: category_request.page.clone(),
        }))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(query_result) => {
                match query_result.to_topic_data() {
                    Some(topic_data) => Ok(Response::SendData(topic_data).response()),
                    None => Ok(Response::ToError(true).response())
                }
            }
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}