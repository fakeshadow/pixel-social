use actix::prelude::Future as Future01;
use actix_web::{
    web::{Data, Json, Query},
    Error, HttpResponse,
};
use futures::{FutureExt, TryFutureExt};

use crate::handler::cache_update::CacheUpdateAddr;
use crate::handler::{auth::UserJwt, cache::MyRedisPool, db::MyPostgresPool};
use crate::model::{
    common::GlobalVars,
    errors::ResError,
    post::Post,
    topic::{QueryType, Topic, TopicQuery, TopicRequest},
};

pub fn add(
    jwt: UserJwt,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    req: Json<TopicRequest>,
    global: Data<GlobalVars>,
    addr: Data<CacheUpdateAddr>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    add_async(jwt, db, cache, req, global, addr)
        .boxed_local()
        .compat()
}

pub async fn add_async(
    jwt: UserJwt,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    req: Json<TopicRequest>,
    global: Data<GlobalVars>,
    addr: Data<CacheUpdateAddr>,
) -> Result<HttpResponse, Error> {
    jwt.check_privilege()?;

    let req = req
        .into_inner()
        .add_user_id(Some(jwt.user_id))
        .check_new()?;

    let t = db.add_topic(&req, global.get_ref()).await?;

    let res = HttpResponse::Ok().json(&t);

    actix::spawn(
        Box::pin(async move { cache.add_topic_send_fail(t, addr.into_inner()).await })
            .boxed_local()
            .compat(),
    );

    Ok(res)
}

pub fn update(
    jwt: UserJwt,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    req: Json<TopicRequest>,
    addr: Data<CacheUpdateAddr>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    update_async(jwt, db, cache, req, addr)
        .boxed_local()
        .compat()
}

async fn update_async(
    jwt: UserJwt,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    req: Json<TopicRequest>,
    addr: Data<CacheUpdateAddr>,
) -> Result<HttpResponse, Error> {
    let req = req
        .into_inner()
        .add_user_id(Some(jwt.user_id))
        .check_update()?;

    let t = db.update_topic(&req).await?;

    let res = HttpResponse::Ok().json(&t);

    update_topic_send_fail(cache, t, addr);

    Ok(res)
}

pub(crate) fn update_topic_send_fail(
    cache: Data<MyRedisPool>,
    t: Vec<Topic>,
    addr: Data<CacheUpdateAddr>,
) {
    actix::spawn(
        Box::pin(async move { cache.update_topic_send_fail(t, addr.into_inner()).await }).compat(),
    );
}

pub fn query_handler(
    req: Query<TopicQuery>,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    query_handler_async(req, db, cache).boxed_local().compat()
}

pub async fn query_handler_async(
    req: Query<TopicQuery>,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
) -> Result<HttpResponse, Error> {
    match req.query_type {
        QueryType::Oldest => {
            let result = cache.get_posts_old(req.topic_id, req.page).await;
            if_query_db(req.topic_id, req.page, db, cache, result).await
        }
        QueryType::Popular => {
            let result = cache.get_posts_pop(req.topic_id, req.page).await;
            if_query_db(req.topic_id, req.page, db, cache, result).await
        }
    }
}

async fn if_query_db(
    tid: u32,
    page: usize,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    result: Result<(Vec<Post>, Vec<u32>), ResError>,
) -> Result<HttpResponse, Error> {
    let mut should_update_u = false;
    let mut should_update_p = false;
    let mut should_update_t = false;

    let (p, mut uids) = match result {
        Ok((p, uids)) => (p, uids),
        Err(e) => {
            if let ResError::IdsFromCache(pids) = e {
                should_update_p = true;
                db.get_posts(&pids).await?
            } else {
                return Err(e.into());
            }
        }
    };

    let (t, mut uid) = if page == 1 {
        match cache.get_topics(vec![tid]).await {
            Ok((t, uid)) => (t, uid),
            Err(e) => {
                if let ResError::IdsFromCache(tids) = e {
                    should_update_t = true;
                    db.get_topics(&tids).await?
                } else {
                    return Err(e.into());
                }
            }
        }
    } else {
        (vec![], vec![])
    };

    uids.append(&mut uid);

    let u = match cache.get_users(uids).await {
        Ok(u) => u,
        Err(e) => {
            if let ResError::IdsFromCache(uids) = e {
                should_update_u = true;
                db.get_users(&uids).await?
            } else {
                vec![]
            }
        }
    };

    if should_update_u {
        let _ = cache.update_users(&u).await;
    };
    if should_update_t {
        let _ = cache.update_topics(&t).await;
    };
    if should_update_p {
        let _ = cache.update_posts(&p).await;
    };

    Ok(HttpResponse::Ok().json(&Topic::attach_users_with_post(t.first(), &p, &u)))
}
