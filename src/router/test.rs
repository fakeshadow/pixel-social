use actix_web::{Error, HttpResponse, web::{Data, Json, Path}};
use futures::Future;

use crate::model::{
    common::{GlobalGuard, PostgresPool,RedisPool},
    topic::{TopicRequest, TopicQuery},
};
use crate::handler::cache::UpdateCacheAsync;

pub fn test_global_var(
    global: Data<GlobalGuard>,
    db: Data<PostgresPool>,
    cache: Data<RedisPool>
) -> impl Future<Item=HttpResponse, Error=Error> {
    TopicQuery::AddTopic(TopicRequest {
        id: None,
        user_id: Some(1),
        category_id: Some(1),
        thumbnail: Some("test thumbnail".to_string()),
        title: Some("test title".to_string()),
        body: Some("test body".to_string()),
        is_locked: None,
    }).into_topic_with_category(db.get_ref().clone(), Some(global.get_ref().clone()))
        .from_err()
        .and_then(move |(c, t)|
            UpdateCacheAsync::AddedTopic(c, t)
                .handler(&cache)
                .then(|_| HttpResponse::Ok().finish()))
}

pub fn test_hello_world() -> HttpResponse {
    HttpResponse::Ok().json("hello world")
}