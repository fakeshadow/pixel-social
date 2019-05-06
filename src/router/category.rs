use futures::{IntoFuture, Future, future::result as ftr};

use actix_web::{web::{Data, Json, Path}, HttpResponse};

use crate::model::{
    errors::ServiceError,
    category::{CategoryRequest, CategoryQuery},
    common::{PostgresPool, RedisPool, QueryOption},
    cache::CacheQuery,
};
use crate::handler::{auth::UserJwt, cache::handle_cache_query};

pub fn get_all_categories(cache: Data<RedisPool>, db: Data<PostgresPool>)
                          -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    handle_cache_query(CacheQuery::GetAllCategories, &cache)
        .into_future()
        .then(move |res| match res {
            Ok(res) => ftr(Ok(res)),
            Err(_) => CategoryQuery::GetAllCategories
                .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
                .into_future()
        })
}

pub fn get_popular(path: Path<(i64)>, cache: Data<RedisPool>, db: Data<PostgresPool>)
                   -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    // ToDo: Add get popular cache query
    let page = path.as_ref();
    CategoryQuery::GetPopular(&page)
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
        .into_future()
}

pub fn get_category(path: Path<(u32, i64)>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                    -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    use crate::model::{cache::PageToCategoryQuery, category::PageToQuery};
    let (id, page) = path.into_inner();
    handle_cache_query(page.to_query_cache(&id), &cache)
        .into_future()
        .then(move |result| match result {
            Ok(res) => ftr(Ok(res)),
            Err(_) => page
                .to_query(&vec![id])
                .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
                .into_future()
        })
}

pub fn get_categories(req: Json<CategoryRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                      -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    req.to_query()
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
        .into_future()
}
