use actix::prelude::Future as Future01;
use actix_web::{
    web::{Data, Json},
    Error, HttpResponse,
};
use futures::future::{FutureExt, TryFutureExt};

use crate::handler::{
    auth::UserJwt,
    cache::MyRedisPool,
    cache_update::CacheUpdateAddr,
    db::{GetStatement, MyPostgresPool, ParseRowStream},
};

use crate::model::{
    common::GlobalVars,
    errors::ResError,
    post::PostRequest,
    topic::{Topic, TopicRequest},
};

pub fn add_topic(
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    global: Data<GlobalVars>,
    addr: Data<CacheUpdateAddr>,
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

    Box::pin(async move {
        crate::router::topic::add_async(jwt, db, cache, Json(req), global, addr).await
    })
    .compat()
}

pub fn add_post(
    global: Data<GlobalVars>,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    addr: Data<CacheUpdateAddr>,
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

    crate::router::post::add_async(jwt, db, cache, Json(req), global, addr)
        .boxed_local()
        .compat()
}

//pub fn test(req: actix_web::HttpRequest) -> impl Future01<Item=HttpResponse, Error=Error> {
//    let token = req.headers()
//        .get("Authorization")
//        .unwrap()
//        .to_str()
//        .unwrap()
//        .rsplitn(2, ' ')
//        .take(0)
//        .next()
//        .unwrap();
//
//    actix_web::web::block(move || logout_query(token.as_str()))
//        .map_err(|err| match err {
//            actix_web::error::BlockingError::Error(service_error) => service_error.into(),
//            actix_web::error::BlockingError::Canceled => {
//                error!("logout: {:?}", err);
//                ServiceError::InternalServerError.into()
//            }
//        })
//        .map(|_| {
//            HttpResponse::Ok().json(LogoutResponse {
//                message: "logout successful".to_string(),
//            })
//        })

//    req.headers()
//        .get("Authorization")
//        .cloned()
//        .ok_or(ServiceError::InternalServerError)
//        .into_future()
//        .and_then(|header| {
//            header
//                .to_str()
//                .map_err(|_| ServiceError::InternalServerError)
//                .and_then(|header_str| {
//                    header_str
//                        .rsplitn(2, ' ')
//                        .take(0)
//                        .next()
//                        .map(|token_str| token_str.to_string())
//                        .ok_or(ServiceError::InternalServerError)
//                })
//        })
//        .and_then(|token| {
//            actix_web::web::block(move || logout_query(token.as_str()))
//                .map_err(|err| match err {
//                    actix_web::error::BlockingError::Error(service_error) => service_error,
//                    actix_web::error::BlockingError::Canceled => {
//                        error!("logout: {:?}", err);
//                        ServiceError::InternalServerError
//                    }
//                })
//                .map(|_| {
//                    HttpResponse::Ok().json(LogoutResponse {
//                        message: "logout successful".to_string(),
//                    })
//                })
//        })
//        .from_err()
//}

pub fn raw(db: Data<MyPostgresPool>) -> impl Future01<Item = HttpResponse, Error = Error> {
    raw_async(db).boxed_local().compat().from_err()
}

pub fn raw_cache(cache: Data<MyRedisPool>) -> impl Future01<Item = HttpResponse, Error = Error> {
    raw_cache_async(cache).boxed_local().compat()
}

async fn raw_async(pool: Data<MyPostgresPool>) -> Result<HttpResponse, ResError> {
    let ids = vec![
        1u32, 11, 9, 20, 3, 5, 2, 6, 19, 8, 9, 10, 12, 13, 14, 15, 16, 17, 18, 4,
    ];

    let pool = pool.get().await?;
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

async fn raw_cache_async(cache: Data<MyRedisPool>) -> Result<HttpResponse, Error> {
    let ids = vec![
        1u32, 11, 9, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 17, 18, 19,
    ];
    let (t, uids) = cache.get_topics(ids).await?;
    let u = cache.get_users(uids).await?;

    Ok(HttpResponse::Ok().json(&Topic::attach_users(&t, &u)))
}
