use std::sync::Arc;

use actix_web::{AsyncResponder, FutureResponse, HttpResponse, ResponseError, State, Json, Path, Request};

use futures::future::{Future, join_all};

use actix_redis::{Command, RedisActor, Error as ARError};

use redis_async::resp::RespValue;

use crate::app::AppState;
use crate::model::{category::*, response::Response};
use crate::handler::auth::UserJwt;
use std::any::Any;
use crate::model::common::GetSelfTimeStamp;

pub fn get_all_categories(state: State<AppState>) -> FutureResponse<HttpResponse> {
    state.db
        .send(CategoryQuery::GetAllCategories)
        .from_err()
        .and_then(move |db_response| match db_response {
            Ok(query_result) => Ok(match_query_result(query_result, state)),
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
            Ok(query_result) => Ok(match_query_result(query_result, state)),
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
            page: Some(page as i64),
        }))
        .from_err()
        .and_then(move |db_response| match db_response {
            Ok(query_result) => {
                Ok(match_query_result(query_result, state))
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
            Ok(query_result) => Ok(match_query_result(query_result, state)),
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}

use std::collections::hash_map::HashMap;
use crate::model::errors::ServiceError;

fn match_query_result(result: CategoryQueryResult, state: State<AppState>)
                      -> HttpResponse {
    match result {
        CategoryQueryResult::GotCategories(categories) => Response::SendData(categories).response(),
        CategoryQueryResult::GotTopics(topics) => {
            let cache = state.cache.clone();

            let category_id = &topics[0].topic.category_id;
            let category_key = format!("category:{}", category_id);


            let mut commands = Vec::with_capacity(20);
            for topic in topics.iter() {
                let time_stamp = topic.get_last_reply_timestamp();
                let stringify = stringify!(topic);
                commands.push(cache.send(Command(resp_array!["ZADD", &category_key, stringify])))
            }

            let info_set = join_all(commands.into_iter());

            info_set
                .and_then(|res: Vec<Result<RespValue, ARError>>|
                    // successful operations return "OK", so confirm that all returned as so
                    if !res.iter().all(|res| match res {
                        Ok(RespValue::SimpleString(x)) if x == "OK" => true,
                        _ => false
                    }) {
                        println!("{}", category_key);
                        match cache.send(Command(resp_array!["ZRANGE", category_key, 0 ,1]))
                            .wait() {
                            Ok(result) => println!("{:?}", result),
                            Err(e) => println!("{:?}", e)
                        }
                        Ok(())
                    } else {
                        println!("failed");
                        Ok(())
                    }
                ).wait();

//

            Response::SendData(topics).response()
        }
        CategoryQueryResult::ModifiedCategory => Response::Modified(true).response()
    }
}