use actix_web::{Error, HttpResponse, web::{Data, Json, Path}};
use futures::{Future, IntoFuture};

use crate::handler::auth::UserJwt;
use crate::model::{
    common::{GlobalGuard, PostgresPool, RedisPool, Response},
    topic::*,
};

pub fn test_global_var(global: Data<GlobalGuard>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                       -> impl Future<Item=HttpResponse, Error=Error> {
    TopicQuery::AddTopic(TopicRequest {
        id: None,
        user_id: Some(1),
        category_id: Some(1),
        thumbnail: Some("test thumbnail".to_string()),
        title: Some("test title".to_string()),
        body: Some("test body".to_string()),
        is_locked: None,
    }).into_topic(&db, Some(global))
        .from_err()
        .and_then(|r| Response::ModifiedTopic.to_res())
}