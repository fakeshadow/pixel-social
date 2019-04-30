use actix_web::{web, Error, HttpResponse, ResponseError};
use futures::{IntoFuture, Future};

use crate::handler::auth::UserJwt;
use crate::model::{
    user::*,
//    cache::*,
    category::*,
    topic::*,
    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool},
    errors::ServiceError,
};
use crate::handler::user::{AsyncDb, async_query};

pub fn test_global_var(
    global_var: web::Data<GlobalGuard>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let topic_query = TopicQuery::AddTopic(&TopicRequest {
        id: None,
        user_id: Some(&1),
        category_id: Some(&1),
        thumbnail: Some("test thumbnail"),
        title: Some("test title"),
        body: Some("test body"),
        is_locked: None,
    });
    let opt = QueryOption::new(Some(&db_pool), None, Some(&global_var));
    topic_query.handle_query(&opt).into_future()
}

pub fn async_test(
    db_pool: web::Data<PostgresPool>,
    cache_pool: web::Data<RedisPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {
//    let (category_id, page) = (1u32, 1);
//
    let opt = QueryOption::new(Some(&db_pool), None, None);
//    let categories = vec![category_id];
//    let category_request = CategoryRequest {
//        categories: &categories,
//        page: &page,
//    };
//    CategoryQuery::GetCategory(&category_request).handle_query(&opt)

//    UserQuery::GetMe(&1).handle_query(&opt).into_future()

    async_query(AsyncDb::GetMe(1), &opt).from_err().and_then(|u| HttpResponse::Ok().json(&u))
}