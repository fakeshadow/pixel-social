use actix_web::{Error, HttpResponse, web::{Data, Json}};
use futures::{
    future::{FutureExt, TryFutureExt},
};
use futures01::Future as Future01;

use crate::handler::{
    auth::UserJwt,
    cache::CacheService,
    db::DatabaseService,
};
use crate::model::{
    common::GlobalVars, errors::ResError, post::PostRequest,
    topic::{Topic, TopicRequest},
};

pub fn add_topic(
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    global: Data<GlobalVars>,
) -> impl Future01<Item=HttpResponse, Error=Error> {
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

    crate::router::topic::add_async(jwt, db, cache, Json(req), global).boxed_local().compat().from_err()
}

pub fn add_post(
    global: Data<GlobalVars>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future01<Item=HttpResponse, Error=Error> {
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

    crate::router::post::add_async(jwt, db, cache, Json(req), global).boxed_local().compat().from_err()
}


pub fn raw(db: Data<DatabaseService>) -> impl Future01<Item=HttpResponse, Error=Error> {
    raw_async(db).boxed_local().compat().from_err()
}

pub fn raw_cache(cache: Data<CacheService>) -> impl Future01<Item=HttpResponse, Error=Error> {
    Box::pin(raw_cache_async(cache)).compat().from_err()
}

async fn raw_async(db: Data<DatabaseService>) -> Result<HttpResponse, ResError> {
    let ids = vec![
        1u32, 11, 9, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 17, 18, 19,
    ];

    let (t, uids) = db.get_topics_with_uid(&ids).await?;
    let u = db.get_users_by_id(&uids).await?;

    Ok(HttpResponse::Ok().json(&Topic::attach_users(&t, &u)))
}

async fn raw_cache_async(cache: Data<CacheService>) -> Result<HttpResponse, ResError> {
    let ids = vec![
        1u32, 20, 11, 9, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 17, 18, 19,
    ];

    let (t, uids) = cache.get_topics_from_ids(ids).await?;
    let u = cache.get_users_from_ids(uids).await?;
    Ok(HttpResponse::Ok().json(&Topic::attach_users(&t, &u)))
}

