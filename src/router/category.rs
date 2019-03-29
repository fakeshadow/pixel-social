use actix_web::{AsyncResponder, FutureResponse, HttpResponse, ResponseError, State, Json, Path};

use futures::{future::result as future_ok, future::Future};

use crate::app::AppState;
use crate::model::{category::*, cache::*, response::Response};

use crate::handler::auth::UserJwt;

pub fn get_all_categories(state: State<AppState>) -> FutureResponse<HttpResponse> {
    state.db
        .send(CategoryQuery::GetAllCategories)
        .from_err()
        .and_then(move |db_response| match db_response {
            Ok(query_result) => Ok(match_query_result(query_result)),
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}

pub fn get_popular((page, state): (Path<(u32)>, State<AppState>))
                   -> FutureResponse<HttpResponse> {
    let page = page.into_inner();
    state.db
        .send(CategoryQuery::GetPopular(page as i64))
        .from_err()
        .and_then(move |db_response| match db_response {
            Ok(query_result) => Ok(match_query_result(query_result)),
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}

pub fn get_category((category_query, state, ): (Path<(u32, u32)>, State<AppState>))
                    -> FutureResponse<HttpResponse> {
    let (category_id, page) = category_query.into_inner();

    /// get cache from redis if there is no result or an error occur then start a query to database
    let cache_query = CacheRequest {
        categories: Some(vec![category_id as i32]),
        page: Some(page as isize),
    };
    let cache_query_result =
        state.cache
            .send(CacheQuery::GetCategory(cache_query))
            .wait();

    match cache_query_result {
        Ok(future_result) => {
            match future_result {
                Ok(cache_query_result) => return Box::new(future_ok(Ok(match_cache_query_result(cache_query_result)))),
                Err(_) => {}
            }
        },
        Err(_) => {}
    };

    let db_query = CategoryRequest {
        categories: Some(vec![category_id as i32]),
        modify_type: None,
        category_id: None,
        category_data: None,
        page: Some(page as i64),
    };
    state.db
        .send(CategoryQuery::GetCategory(db_query))
        .from_err()
        .and_then(move |db_response| match db_response {
            Ok(query_result) => {
                Ok(match_query_result(query_result))
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
        .and_then(move |db_response| match db_response {
            Ok(query_result) => Ok(match_query_result(query_result)),
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}

fn match_query_result(result: CategoryQueryResult)
                      -> HttpResponse {
    match result {
        CategoryQueryResult::GotCategories(categories) => HttpResponse::Ok().json(categories),
        CategoryQueryResult::GotTopics(topics) => HttpResponse::Ok().json(topics),
        CategoryQueryResult::ModifiedCategory => Response::Modified.response()
    }
}

fn match_cache_query_result(result: CacheQueryResult) -> HttpResponse {
    match result {
        CacheQueryResult::Tested(test) => HttpResponse::Ok().json(test),
        CacheQueryResult::GotCategory(category_data) => HttpResponse::Ok().json(category_data)
    }
}