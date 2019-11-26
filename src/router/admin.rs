use actix::prelude::Future as Future01;
use actix_web::{
    web::{Data, Json, Path},
    Error, HttpResponse,
};
use futures::{FutureExt, TryFutureExt};

use crate::handler::cache_update::RedisFailedTaskSender;
use crate::handler::{auth::UserJwt, cache::POOL_REDIS, db::POOL};
use crate::model::{
    category::CategoryRequest, common::Validator, post::PostRequest, topic::TopicRequest,
    user::UpdateRequest,
};

pub fn add_category(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    add_category_async(jwt, req, addr).boxed_local().compat()
}

async fn add_category_async(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().check_new()?;
    let c = POOL.admin_add_category(jwt.privilege, req).await?;

    let res = HttpResponse::Ok().json(&c);

    actix::spawn(
        Box::pin(async move {
            POOL_REDIS
                .add_category_send_fail(c, addr.get_ref().clone())
                .await
        })
        .compat(),
    );

    Ok(res)
}

pub fn update_category(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    update_category_async(jwt, req).boxed_local().compat()
}

async fn update_category_async(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().check_update()?;
    let c = POOL.admin_update_category(jwt.privilege, req).await?;

    let res = HttpResponse::Ok().json(&c);
    let _ = POOL_REDIS.update_categories(&c).await;

    Ok(res)
}

pub fn remove_category(
    jwt: UserJwt,
    id: Path<u32>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    remove_category_async(jwt, id).boxed_local().compat()
}

async fn remove_category_async(jwt: UserJwt, id: Path<u32>) -> Result<HttpResponse, Error> {
    let id = id.into_inner();

    POOL.admin_remove_category(id, jwt.privilege).await?;
    //ToDo: fix remove category cache
    //    let _ = cache.remove_category(id).await?;

    Ok(HttpResponse::Ok().finish())
}

pub fn update_user(
    jwt: UserJwt,
    req: Json<UpdateRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    update_user_async(jwt, req, addr).boxed_local().compat()
}

async fn update_user_async(
    jwt: UserJwt,
    req: Json<UpdateRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().attach_id(None).check_update()?;

    let req = POOL.update_user_check(jwt.privilege, req).await?;
    let u = POOL.update_user(req).await?;

    let res = HttpResponse::Ok().json(&u);

    crate::router::user::update_user_send_fail(u, addr);

    Ok(res)
}

pub fn update_topic(
    jwt: UserJwt,
    req: Json<TopicRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    update_topic_async(jwt, req, addr).boxed_local().compat()
}

async fn update_topic_async(
    jwt: UserJwt,
    req: Json<TopicRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().add_user_id(None).check_update()?;

    let t = POOL.admin_update_topic(jwt.privilege, &req).await?;

    let res = HttpResponse::Ok().json(&t);

    crate::router::topic::update_topic_send_fail(t, addr);

    Ok(res)
}

pub fn update_post(
    jwt: UserJwt,
    req: Json<PostRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    update_post_async(jwt, req, addr).boxed_local().compat()
}

async fn update_post_async(
    jwt: UserJwt,
    req: Json<PostRequest>,
    addr: Data<RedisFailedTaskSender>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().attach_user_id(None).check_update()?;

    let p = POOL.admin_update_post(jwt.privilege, req).await?;

    let res = HttpResponse::Ok().json(&p);

    crate::router::post::update_post_send_fail(p, addr);

    Ok(res)
}
