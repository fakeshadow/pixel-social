use futures::IntoFuture;

use actix_web::{web, HttpResponse};

use crate::model::{
    errors::ServiceError,
    cache::{CacheQuery, CategoryCacheRequest},
    category::{CategoryJson, CategoryRequest, CategoryQuery, CategoryQueryResult},
    common::{PostgresPool, RedisPool, QueryOption, ResponseMessage},
};
use crate::handler::{
    auth::UserJwt,
    category::category_handler,
    cache::{match_cache_query_result, cache_handler},
};

pub fn get_all_categories(
    cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
//    let cache_query = CacheQuery::GetAllCategories;
    let category_query = CategoryQuery::GetAllCategories;
    let opt = QueryOption::new(Some(&db_pool), None,None);


    match_query_result(category_handler(category_query, opt), &cache_pool)
}

pub fn get_popular(
    category_path: web::Path<(u32)>,
    cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let page = category_path.as_ref();
//    let cache_query = CacheQuery::GetPopular(page as i64);
    let category_query = CategoryQuery::GetPopular(*page as i64);
    let opt = QueryOption::new(Some(&db_pool), None,None);


    match_query_result(category_handler(category_query, opt), &cache_pool)
}

pub fn get_category(
    category_path: web::Path<(u32, i64)>,
    db_pool: web::Data<PostgresPool>,
    cache_pool: web::Data<RedisPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let (category_id, page) = category_path.as_ref();

    let categories = vec![*category_id];
    let cache_page = *page as isize;
    let category_request = CategoryCacheRequest {
        categories: &categories,
        page: &cache_page,
    };

    let opt = QueryOption::new(Some(&db_pool), None,None);
    let cache_query = CacheQuery::GetCategory(category_request);

    match match_cache_query_result(cache_handler(cache_query, &cache_pool)) {
        Ok(cache) => Ok(cache),
        Err(_) => {
            let category_request = CategoryRequest {
                categories: &categories,
                page: &page,
            };
            let category_query = CategoryQuery::GetCategory(category_request);

            match_query_result(category_handler(category_query, opt), &cache_pool)
        }
    }
}

pub fn get_categories(
    category_json: web::Json<CategoryJson>,
    db_pool: web::Data<PostgresPool>,
    cache_pool: web::Data<RedisPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let category_request = CategoryRequest {
        categories: &category_json.categories,
        page: &category_json.page,
    };

    let opt = QueryOption::new(Some(&db_pool), None,None);
    let category_query = CategoryQuery::GetCategory(category_request);

    match_query_result(category_handler(category_query, opt), &cache_pool)
}

pub fn match_query_result(
    result: Result<CategoryQueryResult, ServiceError>,
    cache_pool: &web::Data<RedisPool>,
) -> Result<HttpResponse, ServiceError> {
    match result {
        Ok(query_result) => match query_result {
            CategoryQueryResult::GotCategories(categories) => {
                Ok(HttpResponse::Ok().json(categories))
            }
            CategoryQueryResult::GotTopics(topics) => {
                if topics.len() > 0 {
                    let _ignore = cache_handler(CacheQuery::UpdateCategory(&topics), &cache_pool);
                }
                Ok(HttpResponse::Ok().json(topics))
            }
            CategoryQueryResult::UpdatedCategory => {
                Ok(HttpResponse::Ok().json(ResponseMessage::new("Modify Success")))
            }
        },
        Err(e) => Err(e),
    }
}
