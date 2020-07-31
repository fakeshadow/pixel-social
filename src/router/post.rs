use actix_web::{
    web::{Json, Path},
    Error, HttpResponse,
};

use crate::handler::{
    auth::UserJwt, cache::MyRedisPool, cache_update::CacheServiceAddr, data::DataRc,
    db::MyPostgresPool,
};
use crate::model::{
    errors::ResError,
    post::{Post, PostRequest},
};

pub async fn add(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    jwt: UserJwt,
    req: Json<PostRequest>,
    addr: DataRc<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    jwt.check_privilege()?;

    let req = req
        .into_inner()
        .attach_user_id(Some(jwt.user_id))
        .check_new()?;

    let p = db_pool.add_post(req).await?;

    let res = HttpResponse::Ok().json(&p);

    actix_rt::spawn(async move {
        cache_pool
            .add_post_send_fail(p, addr.get_ref().clone())
            .await
    });

    Ok(res)
}

pub async fn update(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    jwt: UserJwt,
    req: Json<PostRequest>,
    addr: DataRc<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    let req = req
        .into_inner()
        .attach_user_id(Some(jwt.user_id))
        .check_update()?;

    let p = db_pool.update_post(req).await?;

    let res = HttpResponse::Ok().json(&p);

    update_post_send_fail(cache_pool, p, addr);

    Ok(res)
}

pub(crate) fn update_post_send_fail(
    cache_pool: DataRc<MyRedisPool>,
    p: Vec<Post>,
    addr: DataRc<CacheServiceAddr>,
) {
    actix_rt::spawn(async move {
        cache_pool
            .update_post_send_fail(p, addr.get_ref().clone())
            .await
    });
}

pub async fn get(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    id: Path<u32>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();

    let mut should_update_p = false;
    let mut should_update_u = false;

    let (p, uids) = match cache_pool.get_posts(vec![id]).await {
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

    let res = HttpResponse::Ok().json(Post::attach_users(&p, &u));

    actix_rt::spawn(async move {
        if should_update_u {
            let _ = cache_pool.update_users(&u).await;
        }
        if should_update_p {
            let _ = cache_pool.update_posts(&p).await;
        }
    });

    Ok(res)
}
