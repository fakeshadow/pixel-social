use actix_web::{
    web::{Json, Query},
    Error, HttpResponse,
};

use crate::handler::{
    auth::UserJwt, cache::MyRedisPool, cache_update::CacheServiceAddr, data::DataRc,
    db::MyPostgresPool,
};
use crate::model::{
    errors::ResError,
    post::Post,
    topic::{QueryType, Topic, TopicQuery, TopicRequest},
};

pub async fn add(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    jwt: UserJwt,
    req: Json<TopicRequest>,
    addr: DataRc<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    jwt.check_privilege()?;

    let req = req
        .into_inner()
        .add_user_id(Some(jwt.user_id))
        .check_new()?;

    let t = db_pool.add_topic(&req).await?;

    let res = HttpResponse::Ok().json(&t);

    actix_rt::spawn(async move {
        cache_pool
            .add_topic_send_fail(t, addr.get_ref().clone())
            .await
    });

    Ok(res)
}

pub async fn update(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    jwt: UserJwt,
    req: Json<TopicRequest>,
    addr: DataRc<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    let req = req
        .into_inner()
        .add_user_id(Some(jwt.user_id))
        .check_update()?;

    let t = db_pool.update_topic(&req).await?;

    let res = HttpResponse::Ok().json(&t);

    update_topic_send_fail(cache_pool, t, addr);

    Ok(res)
}

pub(crate) fn update_topic_send_fail(
    cache_pool: DataRc<MyRedisPool>,
    t: Vec<Topic>,
    addr: DataRc<CacheServiceAddr>,
) {
    actix_rt::spawn(async move {
        cache_pool
            .update_topic_send_fail(t, addr.get_ref().clone())
            .await
    });
}

pub async fn query_handler(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    req: Query<TopicQuery>,
) -> Result<HttpResponse, Error> {
    match req.query_type {
        QueryType::Oldest => {
            let result = cache_pool.get_posts_old(req.topic_id, req.page).await;
            if_query_db(db_pool, cache_pool, req.topic_id, req.page, result).await
        }
        QueryType::Popular => {
            let result = cache_pool.get_posts_pop(req.topic_id, req.page).await;
            if_query_db(db_pool, cache_pool, req.topic_id, req.page, result).await
        }
    }
}

async fn if_query_db(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
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
                db_pool.get_posts(&pids).await?
            } else {
                return Err(e.into());
            }
        }
    };

    let (t, mut uid) = if page == 1 {
        match cache_pool.get_topics(vec![tid]).await {
            Ok((t, uid)) => (t, uid),
            Err(e) => {
                if let ResError::IdsFromCache(tids) = e {
                    should_update_t = true;
                    db_pool.get_topics(&tids).await?
                } else {
                    return Err(e.into());
                }
            }
        }
    } else {
        (vec![], vec![])
    };

    uids.append(&mut uid);

    let u = match cache_pool.get_users(uids).await {
        Ok(u) => u,
        Err(e) => {
            if let ResError::IdsFromCache(uids) = e {
                should_update_u = true;
                db_pool.get_users(&uids).await?
            } else {
                vec![]
            }
        }
    };

    let res = HttpResponse::Ok().json(&Topic::attach_users_with_post(t.first(), &p, &u));

    actix_rt::spawn(async move {
        if should_update_u {
            let _ = cache_pool.update_users(&u).await;
        };
        if should_update_t {
            let _ = cache_pool.update_topics(&t).await;
        };
        if should_update_p {
            let _ = cache_pool.update_posts(&p).await;
        };
    });

    Ok(res)
}
