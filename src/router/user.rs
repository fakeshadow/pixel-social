use actix_web::{
    web::{Data, Json, Path},
    Error, HttpResponse,
};
use futures::{FutureExt, TryFutureExt};
use futures01::Future as Future01;

use crate::handler::{
    auth::UserJwt,
    cache::{CacheService, CheckCacheConn},
    db::DatabaseService,
};
use crate::model::{
    common::Validator,
    user::{UpdateRequest, User},
};

pub fn get(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Path<(u32)>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    get_async(jwt, db, cache, req).boxed_local().compat()
}

async fn get_async(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Path<(u32)>,
) -> Result<HttpResponse, Error> {
    let id = req.into_inner();
    let u = match cache.get_users_from_ids(vec![id]).await {
        Ok(u) => u,
        Err(_) => db.get_users_by_id(&[id]).await?,
    };

    if id == jwt.user_id {
        Ok(HttpResponse::Ok().json(u.first()))
    } else {
        Ok(HttpResponse::Ok().json(u.first().map(|u| u.to_user_ref())))
    }
}

pub fn update(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Json<UpdateRequest>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    update_async(jwt, db, cache, req).boxed_local().compat()
}

async fn update_async(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Json<UpdateRequest>,
) -> Result<HttpResponse, Error> {
    let req = req
        .into_inner()
        .attach_id(Some(jwt.user_id))
        .check_update()?;

    let u = db.check_conn().await?.update_user(req).await?;

    let res = HttpResponse::Ok().json(&u);

    update_user_with_fail_check(cache, u).await;

    Ok(res)
}

pub(crate) async fn update_user_with_fail_check(cache: Data<CacheService>, u: User) {
    let u = vec![u];

    match cache.check_conn().await {
        Ok(opt) => actix::spawn(
            cache
                .if_replace_cache(opt)
                .update_user_return_fail01(u)
                .map_err(move |u| cache.send_failed_user(u)),
        ),
        Err(_) => cache.send_failed_user(u),
    };
}
