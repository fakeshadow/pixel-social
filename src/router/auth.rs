use actix_web::{
    web::{Data, Json, Path},
    Error, HttpResponse,
};

use crate::handler::cache_update::CacheServiceAddr;
use crate::handler::{auth::UserJwt, cache::POOL_REDIS, db::POOL};
use crate::model::{
    common::Validator,
    errors::ResError,
    user::{AuthRequest, UpdateRequest},
};

pub async fn login(req: Json<AuthRequest>) -> Result<HttpResponse, Error> {
    let r = req.into_inner().check_login()?;
    let r = POOL.login(r).await?;
    Ok(HttpResponse::Ok().json(&r))
}

pub async fn register(
    req: Json<AuthRequest>,
    addr: Data<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().check_register()?;

    let u = POOL.register(req).await?;

    let res = HttpResponse::Ok().json(&u);

    // POOL_REDIS.add_activation_mail(u.clone()).await;
    crate::router::user::update_user_send_fail(u, addr);

    Ok(res)
}

pub async fn activate_by_mail(
    req: Path<String>,
    addr: Data<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    let uuid = req.into_inner();

    let uid = POOL_REDIS.get_uid_from_uuid(uuid.as_str()).await?;

    let u = POOL.update_user(UpdateRequest::make_active(uid)).await?;

    let res = HttpResponse::Ok().json(&u);

    // POOL_REDIS.remove_activation_uuid(uuid.as_str()).await;

    crate::router::user::update_user_send_fail(u, addr);

    Ok(res)
}

pub async fn add_activation_mail(jwt: UserJwt) -> Result<HttpResponse, Error> {
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

    // let _ = POOL_REDIS.add_activation_mail(u);
    Ok(HttpResponse::Ok().finish())
}
