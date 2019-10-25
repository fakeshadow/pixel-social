use actix::prelude::Future as Future01;
use actix_web::{
    web::{Data, Json},
    Error, HttpResponse,
};
use futures::future::{FutureExt, TryFutureExt};

use crate::handler::{
    auth::UserJwt,
    cache::POOL_REDIS,
    cache_update::RedisFailedTaskSender,
    db::{GetStatement, ParseRowStream, POOL},
};

use crate::model::{
    errors::ResError,
    post::PostRequest,
    topic::{Topic, TopicRequest},
};

pub fn add_topic(
    addr: Data<RedisFailedTaskSender>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    let req = TopicRequest {
        id: None,
        user_id: Some(1),
        category_id: 1,
        thumbnail: Some("test thumbnail".to_string()),
        title: Some("test title".to_string()),
        body: Some("test body".to_string()),
        is_locked: None,
        is_visible: Some(true),
    };

    let jwt = UserJwt {
        exp: 0,
        user_id: 1,
        privilege: 9,
    };

    Box::pin(async move { crate::router::topic::add_async(jwt, Json(req), addr).await }).compat()
}

pub fn add_post(
    addr: Data<RedisFailedTaskSender>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    let req = PostRequest {
        id: None,
        user_id: Some(1),
        topic_id: Some(1),
        category_id: 1,
        post_id: Some(1),
        post_content: Some("t4265335423646e".to_owned()),
        is_locked: None,
    };

    let jwt = UserJwt {
        exp: 0,
        user_id: 1,
        privilege: 9,
    };

    crate::router::post::add_async(jwt, Json(req), addr)
        .boxed_local()
        .compat()
}

pub fn raw() -> impl Future01<Item = HttpResponse, Error = Error> {
    raw_async().boxed_local().compat().from_err()
}

pub fn raw_cache() -> impl Future01<Item = HttpResponse, Error = Error> {
    raw_cache_async().boxed_local().compat()
}

async fn raw_async() -> Result<HttpResponse, ResError> {
    let ids = vec![
        1u32, 11, 9, 20, 3, 5, 2, 6, 19, 8, 9, 10, 12, 13, 14, 15, 16, 17, 18, 4,
    ];

    let pool = POOL.get().await?;
    let (cli, sts) = &*pool;

    let st = sts.get_statement("topics_by_id")?;

    let (t, mut uids) = cli
        .query_raw(
            st,
            [&ids as &(dyn tokio_postgres::types::ToSql + Sync)]
                .iter()
                .map(|s| *s as _),
        )
        .await?
        .parse_row_with()
        .await?;

    uids.sort();
    uids.dedup();

    let st = sts.get_statement("users_by_id")?;
    let u = cli
        .query_raw(
            st,
            [&uids as &(dyn tokio_postgres::types::ToSql + Sync)]
                .iter()
                .map(|s| *s as _),
        )
        .await?
        .parse_row()
        .await?;

    drop(pool);

    let t = Topic::sort(t, &ids).await;

    Ok(HttpResponse::Ok().json(&Topic::attach_users(&t, &u)))
}

async fn raw_cache_async() -> Result<HttpResponse, Error> {
    let ids = vec![
        1u32, 11, 9, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 17, 18, 19,
    ];
    let (t, uids) = POOL_REDIS.get_topics(ids).await?;
    let u = POOL_REDIS.get_users(uids).await?;

    Ok(HttpResponse::Ok().json(&Topic::attach_users(&t, &u)))
}
