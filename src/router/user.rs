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
    user::{UpdateRequest, User},
};

pub async fn get(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    jwt: UserJwt,
    req: Path<u32>,
) -> Result<HttpResponse, Error> {
    let id = req.into_inner();
    let u = match cache_pool.get_users(vec![id]).await {
        Ok(u) => u,
        Err(_) => db_pool.get_users(&[id]).await?,
    };

    if id == jwt.user_id {
        Ok(HttpResponse::Ok().json(u.first()))
    } else {
        Ok(HttpResponse::Ok().json(u.first().map(|u| u.to_user_ref())))
    }
}

pub async fn update(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    jwt: UserJwt,
    req: Json<UpdateRequest>,
    addr: DataRc<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    let req = req
        .into_inner()
        .attach_id(Some(jwt.user_id))
        .check_update()?;

    let u = db_pool.update_user(req).await?;

    let res = HttpResponse::Ok().json(&u);

    update_user_send_fail(cache_pool, u, addr);

    Ok(res)
}

pub(crate) fn update_user_send_fail(
    cache_pool: DataRc<MyRedisPool>,
    u: Vec<User>,
    addr: DataRc<CacheServiceAddr>,
) {
    actix_rt::spawn(async move {
        cache_pool
            .update_user_send_fail(u, addr.get_ref().clone())
            .await
    });
}
