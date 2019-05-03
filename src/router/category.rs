use futures::{IntoFuture, Future, future::result as ftr};

use actix_web::{web::{Data, Json, Path}, HttpResponse};

use crate::model::{
    errors::ServiceError,
    category::{CategoryJson, CategoryRequest, CategoryQuery},
    common::{PostgresPool, RedisPool, QueryOption},
};
use crate::handler::{auth::UserJwt, cache::{handle_cache_query, CacheQuery}};

pub fn get_all_categories(cache: Data<RedisPool>, db: Data<PostgresPool>)
                          -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    handle_cache_query(CacheQuery::GetAllCategories, &cache)
        .into_future()
        .then(move |res| match res {
            Ok(res) => ftr(Ok(res)),
            Err(_) => {
                CategoryQuery::GetAllCategories
                    .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
                    .into_future()
            }
        })
}

pub fn get_popular(path: Path<(i64)>, cache: Data<RedisPool>, db: Data<PostgresPool>)
                   -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let page = path.as_ref();
    CategoryQuery::GetPopular(&page)
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
        .into_future()
}

pub fn get_category(path: Path<(u32, i64)>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                    -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let (category_id, page) = path.into_inner();
    handle_cache_query(CacheQuery::GetCategory(&category_id, &page), &cache)
        .into_future()
        .then(move |result| match result {
            Ok(res) => ftr(Ok(res)),
            Err(_) => {
                let categories = vec![category_id];
                let category_request = CategoryRequest {
                    categories: &categories,
                    page: &page,
                };
                CategoryQuery::GetCategory(&category_request)
                    .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
                    .into_future()
            }
        })
}

pub fn get_categories(req: Json<CategoryJson>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                      -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let category_request = CategoryRequest {
        categories: &req.categories,
        page: &req.page,
    };
    CategoryQuery::GetCategory(&category_request)
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
        .into_future()
}
