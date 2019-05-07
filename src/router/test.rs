use actix_web::{HttpResponse, web::{Data, Json, Path}};
use futures::{Future, IntoFuture};

use crate::handler::auth::UserJwt;
use crate::model::{
    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool},
    errors::ServiceError,
    topic::*,
};

pub fn test_global_var(global: Data<GlobalGuard>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                       -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    TopicQuery::AddTopic(&TopicRequest {
        id: None,
        user_id: Some(1),
        category_id: Some(1),
        thumbnail: Some("test thumbnail".to_string()),
        title: Some("test title".to_string()),
        body: Some("test body".to_string()),
        is_locked: None,
    }).handle_query(&QueryOption::new(Some(&db), Some(&cache), Some(&global))).into_future()
}