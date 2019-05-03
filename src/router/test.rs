use futures::{IntoFuture, Future};

use actix_web::{web::{Data, Json, Path}, HttpResponse};

use crate::model::{
    topic::*,
    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool},
    errors::ServiceError,
};
use crate::handler::auth::UserJwt;

pub fn test_global_var(global: Data<GlobalGuard>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                       -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let topic_query = TopicQuery::AddTopic(TopicRequest {
        id: None,
        user_id: Some(1),
        category_id: Some(1),
        thumbnail: Some("test thumbnail".to_string()),
        title: Some("test title".to_string()),
        body: Some("test body".to_string()),
        is_locked: None,
    });
    topic_query.handle_query(&QueryOption::new(Some(&db), Some(&cache), Some(&global))).into_future()
}