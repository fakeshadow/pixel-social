use actix::prelude::Future as Future01;
use actix_web::{
    web::{Data, Json, Query},
    Error, HttpResponse, ResponseError,
};
use futures::{FutureExt, TryFutureExt};

use crate::handler::{
    auth::UserJwt,
    cache::{AddToCache, CacheService, CheckCacheConn},
    db::DatabaseService,
};
use crate::model::{
    common::GlobalVars,
    errors::ResError,
    post::Post,
    topic::{QueryType, Topic, TopicQuery, TopicRequest},
};

pub fn add(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Json<TopicRequest>,
    global: Data<GlobalVars>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    add_async(jwt, db, cache, req, global)
        .boxed_local()
        .compat()
}

pub async fn add_async(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Json<TopicRequest>,
    global: Data<GlobalVars>,
) -> Result<HttpResponse, Error> {
    jwt.check_privilege()?;

    let req = req
        .into_inner()
        .attach_user_id(Some(jwt.user_id))
        .check_new()?;

    let t = db
        .check_conn()
        .await?
        .add_topic(&req, global.get_ref())
        .await?;

    let res = HttpResponse::Ok().json(&t);

    match cache.check_cache_conn().await {
        Ok(opt) => {
            actix::spawn(
                cache
                    .if_replace_cache(opt)
                    .add_topic_cache_01(&t)
                    .map_err(move |_| cache.send_failed_topic(t)),
            );
        }
        Err(_) => cache.send_failed_topic(t),
    };

    Ok(res)
}

pub fn update(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Json<TopicRequest>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    update_async(jwt, db, cache, req).boxed_local().compat()
}

async fn update_async(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Json<TopicRequest>,
) -> Result<HttpResponse, Error> {
    let req = req
        .into_inner()
        .attach_user_id(Some(jwt.user_id))
        .check_update()?;

    let t = db.check_conn().await?.update_topic(&req).await?;

    let res = HttpResponse::Ok().json(&t);

    update_topic_with_fail_check(cache, t).await;

    Ok(res)
}

pub(crate) async fn update_topic_with_fail_check(cache: Data<CacheService>, t: Topic) {
    let t = vec![t];
    match cache.check_cache_conn().await {
        Ok(opt) => actix::spawn(
            cache
                .if_replace_cache(opt)
                .update_topic_return_fail(t)
                .map_err(move |t| cache.send_failed_topic_update(t)),
        ),
        Err(_) => cache.send_failed_topic_update(t),
    };
}

pub fn query_handler(
    req: Query<TopicQuery>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    query_handler_async(req, db, cache).boxed_local().compat()
}

pub async fn query_handler_async(
    req: Query<TopicQuery>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
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
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
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
                db.get_posts_with_uid(&pids).await?
            } else {
                return Ok(e.render_response());
            }
        }
    };

    let (t, mut uid) = if page == 1 {
        match cache.get_topics_from_ids(vec![tid]).await {
            Ok((t, uid)) => (t, uid),
            Err(e) => {
                if let ResError::IdsFromCache(tids) = e {
                    should_update_t = true;
                    db.get_topics_with_uid(&tids).await?
                } else {
                    return Ok(e.render_response());
                }
            }
        }
    } else {
        (vec![], vec![])
    };

    uids.append(&mut uid);

    let u = match cache.get_users_from_ids(uids).await {
        Ok(u) => u,
        Err(e) => {
            if let ResError::IdsFromCache(uids) = e {
                should_update_u = true;
                db.get_users_by_id(&uids).await?
            } else {
                vec![]
            }
        }
    };

    if should_update_u {
        cache.update_users(&u);
    };
    if should_update_t {
        cache.update_topics(&t);
    };
    if should_update_p {
        cache.update_posts(&p);
    };

    Ok(HttpResponse::Ok().json(&Topic::attach_users_with_post(t.first(), &p, &u)))
}
