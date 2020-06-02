use actix_web::{
    web::{Data, Json, Query},
    Error, HttpResponse,
};

use crate::handler::{auth::UserJwt, cache::pool_redis, cache_update::CacheServiceAddr, db::pool};
use crate::model::{
    errors::ResError,
    post::Post,
    topic::{QueryType, Topic, TopicQuery, TopicRequest},
};

pub async fn add(
    jwt: UserJwt,
    req: Json<TopicRequest>,
    addr: Data<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    jwt.check_privilege()?;

    let req = req
        .into_inner()
        .add_user_id(Some(jwt.user_id))
        .check_new()?;

    let t = pool().add_topic(&req).await?;

    let res = HttpResponse::Ok().json(&t);

    actix_rt::spawn(pool_redis().add_topic_send_fail(t, addr.get_ref().clone()));

    Ok(res)
}

pub async fn update(
    jwt: UserJwt,
    req: Json<TopicRequest>,
    addr: Data<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    let req = req
        .into_inner()
        .add_user_id(Some(jwt.user_id))
        .check_update()?;

    let t = pool().update_topic(&req).await?;

    let res = HttpResponse::Ok().json(&t);

    update_topic_send_fail(t, addr);

    Ok(res)
}

pub(crate) fn update_topic_send_fail(t: Vec<Topic>, addr: Data<CacheServiceAddr>) {
    actix_rt::spawn(pool_redis().update_topic_send_fail(t, addr.get_ref().clone()));
}

pub async fn query_handler(req: Query<TopicQuery>) -> Result<HttpResponse, Error> {
    match req.query_type {
        QueryType::Oldest => {
            let result = pool_redis().get_posts_old(req.topic_id, req.page).await;
            if_query_db(req.topic_id, req.page, result).await
        }
        QueryType::Popular => {
            let result = pool_redis().get_posts_pop(req.topic_id, req.page).await;
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
                pool().get_posts(&pids).await?
            } else {
                return Err(e.into());
            }
        }
    };

    let (t, mut uid) = if page == 1 {
        match pool_redis().get_topics(vec![tid]).await {
            Ok((t, uid)) => (t, uid),
            Err(e) => {
                if let ResError::IdsFromCache(tids) = e {
                    should_update_t = true;
                    pool().get_topics(&tids).await?
                } else {
                    return Err(e.into());
                }
            }
        }
    } else {
        (vec![], vec![])
    };

    uids.append(&mut uid);

    let u = match pool_redis().get_users(uids).await {
        Ok(u) => u,
        Err(e) => {
            if let ResError::IdsFromCache(uids) = e {
                should_update_u = true;
                pool().get_users(&uids).await?
            } else {
                vec![]
            }
        }
    };

    if should_update_u {
        let _ = pool_redis().update_users(&u).await;
    };
    if should_update_t {
        let _ = pool_redis().update_topics(&t).await;
    };
    if should_update_p {
        let _ = pool_redis().update_posts(&p).await;
    };

    Ok(HttpResponse::Ok().json(&Topic::attach_users_with_post(t.first(), &p, &u)))
}
