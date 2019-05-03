use actix_web::{web, Error, HttpResponse, ResponseError};
use futures::{IntoFuture, Future};

use crate::handler::auth::UserJwt;
use crate::model::{
    user::*,
    category::*,
    topic::*,
    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool},
    errors::ServiceError,
};

pub fn test_global_var(
    global_var: web::Data<GlobalGuard>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let topic_query = TopicQuery::AddTopic(TopicRequest {
        id: None,
        user_id: Some(1),
        category_id: Some(1),
        thumbnail: Some("test thumbnail".to_string()),
        title: Some("test title".to_string()),
        body: Some("test body".to_string()),
        is_locked: None,
    });
    let opt = QueryOption::new(Some(&db_pool), None, Some(&global_var));
    topic_query.handle_query(&opt).into_future()
}