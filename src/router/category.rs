use actix_web::{web, HttpResponse};

use futures::{IntoFuture};

use crate::model::{category::*, cache::*, response::Response};
use crate::model::types::*;
use crate::handler::cache::cache_handler;
use crate::handler::category::category_handler;

use crate::handler::auth::UserJwt;
use crate::model::errors::ServiceError;

pub fn get_all_categories(
    cache: web::Data<RedisPool>,
    db: web::Data<PostgresPool>
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {

    let cache_query = CacheQuery::GetAllCategories;
    let category_query = CategoryQuery::GetAllCategories;

    match_query_result(category_handler(category_query, db))

}

pub fn get_popular(
    page: web::Path<(u32)>,
    cache: web::Data<RedisPool>,
    db: web::Data<PostgresPool>
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {

    let page = page.into_inner();
    let cache_query = CacheQuery::GetPopular(page as i64);
    let category_query = CategoryQuery::GetPopular(page as i64);

    match_query_result(category_handler(category_query, db))
}

pub fn get_category(
    category_query: web::Path<(u32, u32)>,
    db: web::Data<PostgresPool>,
    cache: web::Data<RedisPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {

    let (category_id, page) = category_query.into_inner();
    let cache_query = CacheQuery::GetCategory(CacheRequest {
        categories: Some(vec![category_id as i32]),
        page: Some(page as isize),
    });

    match match_cache_query_result(cache_handler(cache_query, cache)) {
        Ok(cache) => return Ok(cache),
        Err(_) => {
            let category_query = CategoryQuery::GetCategory(CategoryRequest {
                categories: Some(vec![category_id as i32]),
                modify_type: None,
                category_id: None,
                category_data: None,
                page: Some(page as i64),
            });
            match_query_result(category_handler(category_query, db))
        }
    }
}

pub fn get_categories(
    category_request: web::Json<CategoryRequest>,
    db: web::Data<PostgresPool>,
    cache: web::Data<RedisPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {

    let category_query = CategoryQuery::GetCategory(CategoryRequest {
        categories: category_request.categories.clone(),
        modify_type: None,
        category_id: None,
        category_data: None,
        page: category_request.page.clone(),
    });

    match_query_result(category_handler(category_query, db))
}

fn match_query_result(result: Result<CategoryQueryResult, ServiceError>) -> Result<HttpResponse, ServiceError> {
    match result {
        Ok(query_result) => {
            match query_result {
                CategoryQueryResult::GotCategories(categories) => {
                    Ok(HttpResponse::Ok().json(categories))
                }
                CategoryQueryResult::GotTopics(topics) => {
//                    if topics.len() > 0 {
//                        cache.send(CacheQuery::UpdateCategory(topics.clone())).wait();
//                    }
                    Ok(HttpResponse::Ok().json(topics))
                }
                CategoryQueryResult::ModifiedCategory => Ok(Response::Modified.response())
            }
        }
        Err(e) => Err(e)
    }
}

fn match_cache_query_result(result: Result<CacheQueryResult, ServiceError>) -> Result<HttpResponse, ServiceError> {
    match result {
        Ok(query_result) => {
            match query_result {
                CacheQueryResult::Tested(test) => Ok(HttpResponse::Ok().json(test)),
                CacheQueryResult::GotCategory(category_data) => Ok(HttpResponse::Ok().json(category_data)),
                _ => Ok(HttpResponse::Ok().finish())
            }
        }
        Err(e) => Err(e)
    }
}