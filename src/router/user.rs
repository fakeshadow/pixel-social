use actix::prelude::Future as Future01;
use actix_web::{
    web::{Data, Json, Path},
    Error, HttpResponse,
};
use futures::{FutureExt, TryFutureExt};

use crate::handler::cache_update::CacheUpdateAddr;
use crate::handler::{auth::UserJwt, cache::MyRedisPool, db::MyPostgresPool};
use crate::model::{
    common::Validator,
    user::{UpdateRequest, User},
};

pub fn get(
    jwt: UserJwt,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    req: Path<(u32)>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    get_async(jwt, db, cache, req).boxed_local().compat()
}

async fn get_async(
    jwt: UserJwt,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    req: Path<(u32)>,
) -> Result<HttpResponse, Error> {
    let id = req.into_inner();
    let u = match cache.get_users(vec![id]).await {
        Ok(u) => u,
        Err(_) => db.get_users(&[id]).await?,
    };

    if id == jwt.user_id {
        Ok(HttpResponse::Ok().json(u.first()))
    } else {
        Ok(HttpResponse::Ok().json(u.first().map(|u| u.to_user_ref())))
    }
}

pub fn update(
    jwt: UserJwt,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    req: Json<UpdateRequest>,
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
    req: Json<UpdateRequest>,
    addr: Data<CacheUpdateAddr>,
) -> Result<HttpResponse, Error> {
    let req = req
        .into_inner()
        .attach_id(Some(jwt.user_id))
        .check_update()?;

    let u = db.update_user(req).await?;

    let res = HttpResponse::Ok().json(&u);

    update_user_send_fail(cache, u, addr);

    Ok(res)
}

pub(crate) fn update_user_send_fail(
    cache: Data<MyRedisPool>,
    u: Vec<User>,
    addr: Data<CacheUpdateAddr>,
) {
    actix::spawn(
        Box::pin(async move { cache.update_user_send_fail(u, addr.into_inner()).await }).compat(),
    );
}
