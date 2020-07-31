use actix_web::{web::Json, Error, HttpResponse};

use crate::handler::{
    auth::UserJwt,
    cache::MyRedisPool,
    cache_update::CacheServiceAddr,
    data::DataRc,
    db::{GetStatement, MyPostgresPool, ParseRowStream},
};
use crate::model::{
    errors::ResError,
    post::PostRequest,
    topic::{Topic, TopicRequest},
};

pub async fn add_topic(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    addr: DataRc<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
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

    crate::router::topic::add(db_pool, cache_pool, jwt, Json(req), addr).await
}

pub async fn add_post(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    addr: DataRc<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
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

    crate::router::post::add(db_pool, cache_pool, jwt, Json(req), addr).await
}

pub async fn raw(db_pool: DataRc<MyPostgresPool>) -> Result<HttpResponse, ResError> {
    let ids = vec![
        1u32, 11, 9, 20, 3, 5, 2, 6, 19, 8, 9, 10, 12, 13, 14, 15, 16, 17, 18, 4,
    ];

    let pool = db_pool.get().await?;
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

pub async fn raw_cache(cache_pool: DataRc<MyRedisPool>) -> Result<HttpResponse, Error> {
    let ids = vec![
        1u32, 11, 9, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 17, 18, 19,
    ];

    let (t, uids) = cache_pool.get_topics(ids).await?;

    let u = cache_pool.get_users(uids).await?;

    Ok(HttpResponse::Ok().json(&Topic::attach_users(&t, &u)))
}
