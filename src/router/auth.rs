use actix::prelude::Future as Future01;
use actix_web::{
    web::{Data, Json, Path},
    Error, HttpResponse,
};
use futures::{FutureExt, TryFutureExt};

use crate::handler::cache_update::RedisFailedTaskSender;
use crate::handler::{auth::UserJwt, cache::POOL_REDIS, db::POOL};
use crate::model::{
    common::Validator,
    errors::ResError,
    user::{AuthRequest, UpdateRequest},
};

pub fn login(req: Json<AuthRequest>) -> impl Future01<Item = HttpResponse, Error = Error> {
    login_async(req).boxed_local().compat()
}

async fn login_async(req: Json<AuthRequest>) -> Result<HttpResponse, Error> {
    let r = req.into_inner().check_login()?;
    let r = POOL.login(r).await?;
    Ok(HttpResponse::Ok().json(&r))
}

pub fn register(
    req: Json<AuthRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    register_async(req, addr).boxed_local().compat()
}

async fn register_async(
    req: Json<AuthRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().check_register()?;

    let u = POOL.register(req).await?;

    let res = HttpResponse::Ok().json(&u);

    POOL_REDIS.add_activation_mail(u.clone()).await;
    crate::router::user::update_user_send_fail(u, addr);

    Ok(res)
}

pub fn activate_by_mail(
    req: Path<(String)>,
    addr: Data<RedisFailedTaskSender>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    activate_by_mail_async(req, addr).boxed_local().compat()
}

async fn activate_by_mail_async(
    req: Path<(String)>,
    addr: Data<RedisFailedTaskSender>,
) -> Result<HttpResponse, Error> {
    let uuid = req.into_inner();

    let uid = POOL_REDIS.get_uid_from_uuid(uuid.as_str()).await?;

    let u = POOL.update_user(UpdateRequest::make_active(uid)).await?;

    let res = HttpResponse::Ok().json(&u);

    POOL_REDIS.remove_activation_uuid(uuid.as_str()).await;

    crate::router::user::update_user_send_fail(u, addr);

    Ok(res)
}

pub fn add_activation_mail(jwt: UserJwt) -> impl Future01<Item = HttpResponse, Error = Error> {
    add_activation_mail_async(jwt).boxed_local().compat()
}

async fn add_activation_mail_async(jwt: UserJwt) -> Result<HttpResponse, Error> {
    let u = match POOL_REDIS.get_users(vec![jwt.user_id]).await {
        Ok(u) => u,
        Err(e) => {
            if let ResError::IdsFromCache(ids) = e {
                POOL.get_users(&ids).await?
            } else {
                return Err(e.into());
            }
        }
    };

    let _ = POOL_REDIS.add_activation_mail(u);
    Ok(HttpResponse::Ok().finish())
}
