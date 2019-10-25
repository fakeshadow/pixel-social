use actix::prelude::Future as Future01;
use actix_web::{
    web::{Data, Json, Path},
    Error, HttpResponse,
};
use futures::{FutureExt, TryFutureExt};

use crate::handler::cache_update::RedisFailedTaskSender;
use crate::handler::{auth::UserJwt, cache::POOL_REDIS, db::POOL};
use crate::model::{
    errors::ResError,
    post::{Post, PostRequest},
};

pub fn add(
    jwt: UserJwt,
    req: Json<PostRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    add_async(jwt, req, addr).boxed_local().compat()
}

pub async fn add_async(
    jwt: UserJwt,
    req: Json<PostRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> Result<HttpResponse, Error> {
    jwt.check_privilege()?;

    let req = req
        .into_inner()
        .attach_user_id(Some(jwt.user_id))
        .check_new()?;

    let p = POOL.add_post(req).await?;

    let res = HttpResponse::Ok().json(&p);

    actix::spawn(
        Box::pin(async move {
            POOL_REDIS
                .add_post_send_fail(p, addr.get_ref().clone())
                .await
        })
        .boxed_local()
        .compat(),
    );

    Ok(res)
}

pub fn update(
    jwt: UserJwt,
    req: Json<PostRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    update_async(jwt, req, addr).boxed_local().compat()
}

async fn update_async(
    jwt: UserJwt,
    req: Json<PostRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> Result<HttpResponse, Error> {
    let req = req
        .into_inner()
        .attach_user_id(Some(jwt.user_id))
        .check_update()?;

    let p = POOL.update_post(req).await?;

    let res = HttpResponse::Ok().json(&p);

    update_post_send_fail(p, addr);

    Ok(res)
}

pub(crate) fn update_post_send_fail(p: Vec<Post>, addr: Data<RedisFailedTaskSender>) {
    actix::spawn(
        Box::pin(async move {
            POOL_REDIS
                .update_post_send_fail(p, addr.get_ref().clone())
                .await
        })
        .compat(),
    );
}

pub fn get(id: Path<u32>) -> impl Future01<Item = HttpResponse, Error = Error> {
    get_async(id).boxed_local().compat()
}

async fn get_async(id: Path<u32>) -> Result<HttpResponse, Error> {
    let id = id.into_inner();

    let mut should_update_p = false;
    let mut should_update_u = false;

    let (p, uids) = match POOL_REDIS.get_posts(vec![id]).await {
        Ok((p, uids)) => (p, uids),
        Err(e) => {
            if let ResError::IdsFromCache(pids) = e {
                should_update_p = true;
                POOL.get_posts(&pids).await?
            } else {
                return Err(e.into());
            }
        }
    };

    let u = match POOL_REDIS.get_users(uids).await {
        Ok(u) => u,
        Err(e) => {
            if let ResError::IdsFromCache(uids) = e {
                should_update_u = true;
                POOL.get_users(&uids).await?
            } else {
                vec![]
            }
        }
    };

    if should_update_u {
        let _ = POOL_REDIS.update_users(&u).await;
    }
    if should_update_p {
        let _ = POOL_REDIS.update_posts(&p).await;
    }

    Ok(HttpResponse::Ok().json(Post::attach_users(&p, &u)))
}
