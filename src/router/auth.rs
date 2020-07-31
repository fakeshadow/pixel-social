use actix_web::{
    web::{Json, Path},
    Error, HttpResponse,
};

use crate::handler::{
    auth::UserJwt, cache::MyRedisPool, cache_update::CacheServiceAddr, data::DataRc,
    db::MyPostgresPool,
};
use crate::model::{
    common::Validator,
    errors::ResError,
    user::{AuthRequest, UpdateRequest},
};

pub async fn login(
    db_pool: DataRc<MyPostgresPool>,
    req: Json<AuthRequest>,
) -> Result<HttpResponse, Error> {
    let r = req.into_inner().check_login()?;
    let r = db_pool.login(r).await?;
    Ok(HttpResponse::Ok().json(&r))
}

pub async fn register(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    req: Json<AuthRequest>,
    addr: DataRc<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().check_register()?;

    let u = db_pool.register(req).await?;

    let res = HttpResponse::Ok().json(&u);

    // pool_redis().add_activation_mail(u.clone()).await;
    crate::router::user::update_user_send_fail(cache_pool, u, addr);

    Ok(res)
}

pub async fn activate_by_mail(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    req: Path<String>,
    addr: DataRc<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    let uuid = req.into_inner();

    let uid = cache_pool.get_uid_from_uuid(uuid.as_str()).await?;

    let u = db_pool.update_user(UpdateRequest::make_active(uid)).await?;

    let res = HttpResponse::Ok().json(&u);

    // cache_pool.remove_activation_uuid(uuid.as_str()).await;

    crate::router::user::update_user_send_fail(cache_pool, u, addr);

    Ok(res)
}

pub async fn add_activation_mail(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    jwt: UserJwt,
) -> Result<HttpResponse, Error> {
    let u = match cache_pool.get_users(vec![jwt.user_id]).await {
        Ok(u) => u,
        Err(e) => {
            if let ResError::IdsFromCache(ids) = e {
                db_pool.get_users(&ids).await?
            } else {
                return Err(e.into());
            }
        }
    };

    // let _ = cache_pool.add_activation_mail(u);
    Ok(HttpResponse::Ok().finish())
}
