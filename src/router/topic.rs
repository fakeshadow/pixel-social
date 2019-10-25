use actix::prelude::Future as Future01;
use actix_web::{
    web::{Data, Json, Query},
    Error, HttpResponse,
};
use futures::{FutureExt, TryFutureExt};

use crate::handler::cache_update::RedisFailedTaskSender;
use crate::handler::{auth::UserJwt, cache::POOL_REDIS, db::POOL};
use crate::model::{
    errors::ResError,
    post::Post,
    topic::{QueryType, Topic, TopicQuery, TopicRequest},
};

pub fn add(
    jwt: UserJwt,
    req: Json<TopicRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    add_async(jwt, req, addr).boxed_local().compat()
}

pub async fn add_async(
    jwt: UserJwt,
    req: Json<TopicRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> Result<HttpResponse, Error> {
    jwt.check_privilege()?;

    let req = req
        .into_inner()
        .add_user_id(Some(jwt.user_id))
        .check_new()?;

    let t = POOL.add_topic(&req).await?;

    let res = HttpResponse::Ok().json(&t);

    actix::spawn(
        Box::pin(async move {
            POOL_REDIS
                .add_topic_send_fail(t, addr.get_ref().clone())
                .await
        })
        .boxed_local()
        .compat(),
    );

    Ok(res)
}

pub fn update(
    jwt: UserJwt,
    req: Json<TopicRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    update_async(jwt, req, addr).boxed_local().compat()
}

async fn update_async(
    jwt: UserJwt,
    req: Json<TopicRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> Result<HttpResponse, Error> {
    let req = req
        .into_inner()
        .add_user_id(Some(jwt.user_id))
        .check_update()?;

    let t = POOL.update_topic(&req).await?;

    let res = HttpResponse::Ok().json(&t);

    update_topic_send_fail(t, addr);

    Ok(res)
}

pub(crate) fn update_topic_send_fail(t: Vec<Topic>, addr: Data<RedisFailedTaskSender>) {
    actix::spawn(
        Box::pin(async move {
            POOL_REDIS
                .update_topic_send_fail(t, addr.get_ref().clone())
                .await
        })
        .compat(),
    );
}

pub fn query_handler(req: Query<TopicQuery>) -> impl Future01<Item = HttpResponse, Error = Error> {
    query_handler_async(req).boxed_local().compat()
}

pub async fn query_handler_async(req: Query<TopicQuery>) -> Result<HttpResponse, Error> {
    match req.query_type {
        QueryType::Oldest => {
            let result = POOL_REDIS.get_posts_old(req.topic_id, req.page).await;
            if_query_db(req.topic_id, req.page, result).await
        }
        QueryType::Popular => {
            let result = POOL_REDIS.get_posts_pop(req.topic_id, req.page).await;
            if_query_db(req.topic_id, req.page, result).await
        }
    }
}

async fn if_query_db(
    tid: u32,
    page: usize,
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
                POOL.get_posts(&pids).await?
            } else {
                return Err(e.into());
            }
        }
    };

    let (t, mut uid) = if page == 1 {
        match POOL_REDIS.get_topics(vec![tid]).await {
            Ok((t, uid)) => (t, uid),
            Err(e) => {
                if let ResError::IdsFromCache(tids) = e {
                    should_update_t = true;
                    POOL.get_topics(&tids).await?
                } else {
                    return Err(e.into());
                }
            }
        }
    } else {
        (vec![], vec![])
    };

    uids.append(&mut uid);

    let u = match POOL_REDIS.get_users(uids).await {
        Ok(u) => u,
        Err(e) => {
            if let ResError::IdsFromCache(uids) = e {
                should_update_u = true;
                POOL.get_users(&uids).await?
            } else {
                vec![]
            }
        }
    };

    if should_update_u {
        let _ = POOL_REDIS.update_users(&u).await;
    };
    if should_update_t {
        let _ = POOL_REDIS.update_topics(&t).await;
    };
    if should_update_p {
        let _ = POOL_REDIS.update_posts(&p).await;
    };

    Ok(HttpResponse::Ok().json(&Topic::attach_users_with_post(t.first(), &p, &u)))
}
