use actix::prelude::Future as Future01;
use actix_web::{
    web::{Data, Json, Path},
    Error, HttpResponse,
};
use futures::{FutureExt, TryFutureExt};

use crate::handler::cache_update::CacheUpdateAddr;
use crate::handler::{auth::UserJwt, cache::MyRedisPool, db::MyPostgresPool};
use crate::model::{
    common::{GlobalVars, Validator},
    errors::ResError,
    user::{AuthRequest, UpdateRequest},
};

pub fn login(
    db: Data<MyPostgresPool>,
    req: Json<AuthRequest>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    login_async(db, req).boxed_local().compat()
}

async fn login_async(
    db: Data<MyPostgresPool>,
    req: Json<AuthRequest>,
) -> Result<HttpResponse, Error> {
    let r = req.into_inner().check_login()?;
    let r = db.login(r).await?;
    Ok(HttpResponse::Ok().json(&r))
}

pub fn register(
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    global: Data<GlobalVars>,
    req: Json<AuthRequest>,
    addr: Data<CacheUpdateAddr>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    register_async(db, cache, global, req, addr)
        .boxed_local()
        .compat()
}

async fn register_async(
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    global: Data<GlobalVars>,
    req: Json<AuthRequest>,
    addr: Data<CacheUpdateAddr>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().check_register()?;

    let u = db.register(req, global.get_ref()).await?;

    let res = HttpResponse::Ok().json(&u);

    cache.add_activation_mail(u.clone()).await;
    crate::router::user::update_user_send_fail(cache, u, addr);

    Ok(res)
}

pub fn activate_by_mail(
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    req: Path<(String)>,
    addr: Data<CacheUpdateAddr>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    activate_by_mail_async(db, cache, req, addr)
        .boxed_local()
        .compat()
}

async fn activate_by_mail_async(
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    req: Path<(String)>,
    addr: Data<CacheUpdateAddr>,
) -> Result<HttpResponse, Error> {
    let uuid = req.into_inner();

    let uid = cache.get_uid_from_uuid(uuid.as_str()).await?;

    let u = db.update_user(UpdateRequest::make_active(uid)).await?;

    let res = HttpResponse::Ok().json(&u);

    cache.remove_activation_uuid(uuid.as_str()).await;

    crate::router::user::update_user_send_fail(cache, u, addr);

    Ok(res)
}

pub fn add_activation_mail(
    jwt: UserJwt,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    add_activation_mail_async(jwt, db, cache)
        .boxed_local()
        .compat()
}

async fn add_activation_mail_async(
    jwt: UserJwt,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
) -> Result<HttpResponse, Error> {
    let mut u = match cache.get_users(vec![jwt.user_id]).await {
        Ok(u) => u,
        Err(e) => {
            if let ResError::IdsFromCache(ids) = e {
                db.get_users(&ids).await?
            } else {
                return Err(e.into());
            }
        }
    };

    match u.pop() {
        Some(u) => {
            let _ = cache.add_activation_mail(u);
            Ok(HttpResponse::Ok().finish())
        }
        None => Err(ResError::BadRequest.into()),
    }
}
