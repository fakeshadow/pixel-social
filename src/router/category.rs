use actix_web::{web, HttpResponse};
use futures::IntoFuture;

use crate::model::{
    category::*,
    cache::*,
    errors::ServiceError,
    common::{ResponseMessage, PostgresPool, RedisPool},
};
use crate::handler::{
    auth::UserJwt,
    cache::cache_handler,
    category::category_handler,
};

pub fn get_all_categories(
    cache: web::Data<RedisPool>,
    db: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let cache_query = CacheQuery::GetAllCategories;
    let category_query = CategoryQuery::GetAllCategories;

    match_query_result(category_handler(category_query, &db), &cache)
}

pub fn get_popular(
    page: web::Path<(u32)>,
    cache: web::Data<RedisPool>,
    db: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let page = page.into_inner();
    let cache_query = CacheQuery::GetPopular(page as i64);
    let category_query = CategoryQuery::GetPopular(page as i64);

    match_query_result(category_handler(category_query, &db), &cache)
}

pub fn get_category(
    category_query: web::Path<(u32, u32)>,
    db: web::Data<PostgresPool>,
    cache: web::Data<RedisPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let (category_id, page) = category_query.into_inner();

    let categories = vec![category_id];
    let cache_page = page as isize;
    let category_request = CacheRequest {
        categories: &categories,
        page: &cache_page,
    };

    let cache_query = CacheQuery::GetCategory(category_request);

    match match_cache_query_result(cache_handler(cache_query, &cache)) {
        Ok(cache) => return Ok(cache),
        Err(_) => {
            let db_page = page as i64;
            let category_request = CategoryRequest {
                categories: &categories,
                page: &db_page,
            };
            let category_query = CategoryQuery::GetCategory(category_request);

//            println!("prepare to update cache");
            match_query_result(category_handler(category_query, &db), &cache)
        }
    }
}

pub fn get_categories(
    category_json: web::Json<CategoryJson>,
    db: web::Data<PostgresPool>,
    cache: web::Data<RedisPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let category_request = CategoryRequest {
        categories: &category_json.categories,
        page: &category_json.page,
    };
    let category_query = CategoryQuery::GetCategory(category_request);

    match_query_result(category_handler(category_query, &db), &cache)
}

fn match_query_result(result: Result<CategoryQueryResult, ServiceError>, cache: &web::Data<RedisPool>) -> Result<HttpResponse, ServiceError> {
    match result {
        Ok(query_result) => {
            match query_result {
                CategoryQueryResult::GotCategories(categories) => {
                    Ok(HttpResponse::Ok().json(categories))
                }
                CategoryQueryResult::GotTopics(topics) => {
                    if topics.len() > 0 {
                        cache_handler(CacheQuery::UpdateCategory(topics.clone()), &cache);
                    }
                    Ok(HttpResponse::Ok().json(topics))
                }
                CategoryQueryResult::ModifiedCategory => Ok(HttpResponse::Ok().json(ResponseMessage::new("Modify Success")))
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