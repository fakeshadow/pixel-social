use actix_web::{
    web::{Data, Json, Path},
    Error, HttpResponse,
};

use crate::handler::{auth::UserJwt, cache::pool_redis, cache_update::CacheServiceAddr, db::pool};
use crate::model::{
    category::CategoryRequest, common::Validator, post::PostRequest, topic::TopicRequest,
    user::UpdateRequest,
};

pub async fn add_category(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    addr: Data<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().check_new()?;
    let c = pool().admin_add_category(jwt.privilege, req).await?;

    let res = HttpResponse::Ok().json(&c);

    actix::spawn(pool_redis().add_category_send_fail(c, addr.get_ref().clone()));

    Ok(res)
}

pub async fn update_category(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().check_update()?;
    let c = pool().admin_update_category(jwt.privilege, req).await?;

    let res = HttpResponse::Ok().json(&c);
    let _ = pool_redis().update_categories(&c).await;

    Ok(res)
}

pub async fn remove_category(jwt: UserJwt, id: Path<u32>) -> Result<HttpResponse, Error> {
    let id = id.into_inner();

    pool().admin_remove_category(id, jwt.privilege).await?;
    //ToDo: fix remove category cache
    //    let _ = cache.remove_category(id).await?;

    Ok(HttpResponse::Ok().finish())
}

pub async fn update_user(
    jwt: UserJwt,
    req: Json<UpdateRequest>,
    addr: Data<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().attach_id(None).check_update()?;

    let req = pool().update_user_check(jwt.privilege, req).await?;
    let u = pool().update_user(req).await?;

    let res = HttpResponse::Ok().json(&u);

    crate::router::user::update_user_send_fail(u, addr);

    Ok(res)
}

pub async fn update_topic(
    jwt: UserJwt,
    req: Json<TopicRequest>,
    addr: Data<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().add_user_id(None).check_update()?;

    let t = pool().admin_update_topic(jwt.privilege, &req).await?;

    let res = HttpResponse::Ok().json(&t);

    crate::router::topic::update_topic_send_fail(t, addr);

    Ok(res)
}

pub async fn update_post(
    jwt: UserJwt,
    req: Json<PostRequest>,
    addr: Data<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().attach_user_id(None).check_update()?;

    let p = pool().admin_update_post(jwt.privilege, req).await?;

    let res = HttpResponse::Ok().json(&p);

    crate::router::post::update_post_send_fail(p, addr);

    Ok(res)
}
