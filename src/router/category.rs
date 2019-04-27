use futures::IntoFuture;

use actix_web::{web, HttpResponse};

use crate::model::{
    errors::ServiceError,
//    cache::{CacheQuery, CategoryCacheRequest},
    category::{CategoryJson, CategoryRequest, CategoryQuery},
    common::{PostgresPool, RedisPool, QueryOption, ResponseMessage},
};
use crate::handler::auth::UserJwt;

pub fn get_all_categories(
    cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);
    CategoryQuery::GetAllCategories.handle_query(&opt).into_future()
}

pub fn get_popular(
    category_path: web::Path<(i64)>,
    cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let page = category_path.as_ref();
    let opt = QueryOption::new(Some(&db_pool), None, None);

    CategoryQuery::GetPopular(&page).handle_query(&opt).into_future()
}

pub fn get_category(
    category_path: web::Path<(u32, i64)>,
    db_pool: web::Data<PostgresPool>,
    cache_pool: web::Data<RedisPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let (category_id, page) = category_path.as_ref();

    let opt = QueryOption::new(Some(&db_pool), None, None);
    let categories = vec![*category_id];
    let category_request = CategoryRequest {
        categories: &categories,
        page: &page,
    };
    CategoryQuery::GetCategory(&category_request).handle_query(&opt).into_future()
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

    CategoryQuery::GetCategory(&category_request).handle_query(&opt).into_future()
}
