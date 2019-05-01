use futures::{IntoFuture, Future, future::result as ftr};

use actix_web::{web, HttpResponse, Error, Either};

use crate::model::{
    errors::ServiceError,
//    cache::{CacheQuery, CategoryCacheRequest},
    category::{CategoryJson, CategoryRequest, CategoryQuery},
    common::{PostgresPool, RedisPool, QueryOption},
};
use crate::handler::auth::UserJwt;
use crate::handler::cache::{get_topics_cache};

pub fn get_all_categories(
    cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), Some(&cache_pool), None);
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
    let (category_id, page) = category_path.into_inner();

    get_topics_cache(&category_id, &page, &cache_pool)
        .into_future()
        .then(move |result| match result {
            Ok(result) => ftr(Ok(HttpResponse::Ok().json(result))),
            Err(_) => {
                let opt = QueryOption::new(Some(&db_pool), Some(&cache_pool), None);
                let categories = vec![category_id];
                let category_request = CategoryRequest {
                    categories: &categories,
                    page: &page,
                };
                CategoryQuery::GetCategory(&category_request).handle_query(&opt).into_future()
            }
        }).from_err()
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

    let opt = QueryOption::new(Some(&db_pool), Some(&cache_pool), None);

    CategoryQuery::GetCategory(&category_request).handle_query(&opt).into_future()
}
