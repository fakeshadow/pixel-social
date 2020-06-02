use actix_web::{
    web::{Data, Json, Path},
    Error, HttpResponse,
};

use crate::handler::cache_update::CacheServiceAddr;
use crate::handler::{auth::UserJwt, cache::pool_redis, db::pool};
use crate::model::{
    errors::ResError,
    post::{Post, PostRequest},
};

pub async fn add(
    jwt: UserJwt,
    req: Json<PostRequest>,
    addr: Data<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    jwt.check_privilege()?;

    let req = req
        .into_inner()
        .attach_user_id(Some(jwt.user_id))
        .check_new()?;

    let p = pool().add_post(req).await?;

    let res = HttpResponse::Ok().json(&p);

    actix_rt::spawn(pool_redis().add_post_send_fail(p, addr.get_ref().clone()));

    Ok(res)
}

pub async fn update(
    jwt: UserJwt,
    req: Json<PostRequest>,
    addr: Data<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    let req = req
        .into_inner()
        .attach_user_id(Some(jwt.user_id))
        .check_update()?;

    let p = pool().update_post(req).await?;

    let res = HttpResponse::Ok().json(&p);

    update_post_send_fail(p, addr);

    Ok(res)
}

pub(crate) fn update_post_send_fail(p: Vec<Post>, addr: Data<CacheServiceAddr>) {
    actix_rt::spawn(pool_redis().update_post_send_fail(p, addr.get_ref().clone()));
}

pub async fn get(id: Path<u32>) -> Result<HttpResponse, Error> {
    let id = id.into_inner();

    let mut should_update_p = false;
    let mut should_update_u = false;

    let (p, uids) = match pool_redis().get_posts(vec![id]).await {
        Ok((p, uids)) => (p, uids),
        Err(e) => {
            if let ResError::IdsFromCache(pids) = e {
                should_update_p = true;
                pool().get_posts(&pids).await?
            } else {
                return Err(e.into());
            }
        }
    };

    let u = match pool_redis().get_users(uids).await {
        Ok(u) => u,
        Err(e) => {
            if let ResError::IdsFromCache(uids) = e {
                should_update_u = true;
                pool().get_users(&uids).await?
            } else {
                vec![]
            }
        }
    };

    if should_update_u {
        let _ = pool_redis().update_users(&u).await;
    }
    if should_update_p {
        let _ = pool_redis().update_posts(&p).await;
    }

    Ok(HttpResponse::Ok().json(Post::attach_users(&p, &u)))
}
