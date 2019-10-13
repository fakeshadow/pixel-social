use actix::prelude::Future as Future01;
use actix_web::{
    web::{Data, Json, Path},
    Error, HttpResponse,
};
use futures::{FutureExt, TryFutureExt};

use crate::handler::cache_update::CacheUpdateAddr;
use crate::handler::{auth::UserJwt, cache::MyRedisPool, db::MyPostgresPool};
use crate::model::{
    common::GlobalVars,
    errors::ResError,
    post::{Post, PostRequest},
};

pub fn add(
    jwt: UserJwt,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    req: Json<PostRequest>,
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
    req: Json<PostRequest>,
    global: Data<GlobalVars>,
    addr: Data<CacheUpdateAddr>,
) -> Result<HttpResponse, Error> {
    jwt.check_privilege()?;

    let req = req
        .into_inner()
        .attach_user_id(Some(jwt.user_id))
        .check_new()?;

    let p = db.add_post(req, global.get_ref()).await?;

    let res = HttpResponse::Ok().json(&p);

    actix::spawn(
        Box::pin(async move { cache.add_post_send_fail(p, addr.into_inner()).await })
            .boxed_local()
            .compat(),
    );

    Ok(res)
}

pub fn update(
    jwt: UserJwt,
    req: Json<PostRequest>,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    addr: Data<CacheUpdateAddr>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    update_async(jwt, req, db, cache, addr)
        .boxed_local()
        .compat()
}

async fn update_async(
    jwt: UserJwt,
    req: Json<PostRequest>,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    addr: Data<CacheUpdateAddr>,
) -> Result<HttpResponse, Error> {
    let req = req
        .into_inner()
        .attach_user_id(Some(jwt.user_id))
        .check_update()?;

    let p = db.update_post(req).await?;

    let res = HttpResponse::Ok().json(&p);

    update_post_send_fail(cache, p, addr);

    Ok(res)
}

pub(crate) fn update_post_send_fail(
    cache: Data<MyRedisPool>,
    p: Vec<Post>,
    addr: Data<CacheUpdateAddr>,
) {
    actix::spawn(
        Box::pin(async move { cache.update_post_send_fail(p, addr.into_inner()).await }).compat(),
    );
}

pub fn get(
    id: Path<u32>,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    get_async(id, db, cache).boxed_local().compat()
}

async fn get_async(
    id: Path<u32>,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();

    let mut should_update_p = false;
    let mut should_update_u = false;

    let (p, uids) = match cache.get_posts(vec![id]).await {
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
    }
    if should_update_p {
        let _ = cache.update_posts(&p).await;
    }

    Ok(HttpResponse::Ok().json(Post::attach_users(&p, &u)))
}
