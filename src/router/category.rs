use futures::IntoFuture;

use actix_web::{web, HttpResponse};

use crate::model::{
    errors::ServiceError,
    cache::{CacheQuery, CategoryCacheRequest},
    category::{CategoryJson, CategoryRequest, CategoryQuery},
    common::{PostgresPool, RedisPool, QueryOption, ResponseMessage},
};
use crate::handler::{
    auth::UserJwt,
    cache::{match_cache_query_result, cache_handler},
};

pub fn get_all_categories(
    cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);
    Ok(CategoryQuery::GetAllCategories.handle_query(&opt)?.to_response())
}

pub fn get_popular(
    category_path: web::Path<(u32)>,
    cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let page = category_path.as_ref();
//    let cache_query = CacheQuery::GetPopular(page as i64);
    let opt = QueryOption::new(Some(&db_pool), None, None);

    Ok(CategoryQuery::GetPopular(*page as i64).handle_query(&opt)?.to_response())
}

pub fn get_category(
    category_path: web::Path<(u32, i64)>,
    db_pool: web::Data<PostgresPool>,
    cache_pool: web::Data<RedisPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let (category_id, page) = category_path.as_ref();

    let opt = QueryOption::new(Some(&db_pool), None, None);
    let categories = vec![*category_id];
//    let cache_page = *page as isize;
//    let category_request = CategoryCacheRequest {
//        categories: &categories,
//        page: &cache_page,
//    };
    let category_request = CategoryRequest {
        categories: &categories,
        page: &page,
    };
    Ok(CategoryQuery::GetCategory(&category_request).handle_query(&opt)?.to_response())
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

    let opt = QueryOption::new(Some(&db_pool), None, None);

    Ok(CategoryQuery::GetCategory(&category_request).handle_query(&opt)?.to_response())
}
