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
    user::{UpdateRequest, User},
};

pub fn get(jwt: UserJwt, req: Path<(u32)>) -> impl Future01<Item = HttpResponse, Error = Error> {
    get_async(jwt, req).boxed_local().compat()
}

async fn get_async(jwt: UserJwt, req: Path<(u32)>) -> Result<HttpResponse, Error> {
    let id = req.into_inner();
    let u = match POOL_REDIS.get_users(vec![id]).await {
        Ok(u) => u,
        Err(_) => POOL.get_users(&[id]).await?,
    };

    if id == jwt.user_id {
        Ok(HttpResponse::Ok().json(u.first()))
    } else {
        Ok(HttpResponse::Ok().json(u.first().map(|u| u.to_user_ref())))
    }
}

pub fn update(
    jwt: UserJwt,
    req: Json<UpdateRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    update_async(jwt, req, addr).boxed_local().compat()
}

async fn update_async(
    jwt: UserJwt,
    req: Json<UpdateRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> Result<HttpResponse, Error> {
    let req = req
        .into_inner()
        .attach_id(Some(jwt.user_id))
        .check_update()?;

    let u = POOL.update_user(req).await?;

    let res = HttpResponse::Ok().json(&u);

    update_user_send_fail(u, addr);

    Ok(res)
}

pub(crate) fn update_user_send_fail(u: Vec<User>, addr: Data<RedisFailedTaskSender>) {
    actix::spawn(
        Box::pin(async move {
            POOL_REDIS
                .update_user_send_fail(u, addr.get_ref().clone())
                .await
        })
        .compat(),
    );
}
