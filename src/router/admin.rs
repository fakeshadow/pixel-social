use actix_web::{
    web::{Json, Path},
    Error, HttpResponse,
};

use crate::handler::{
    auth::UserJwt, cache::MyRedisPool, cache_update::CacheServiceAddr, data::DataRc,
    db::MyPostgresPool,
};
use crate::model::{
    category::CategoryRequest, common::Validator, post::PostRequest, topic::TopicRequest,
    user::UpdateRequest,
};

pub async fn add_category(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    addr: DataRc<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().check_new()?;
    let c = db_pool.admin_add_category(jwt.privilege, req).await?;

    let res = HttpResponse::Ok().json(&c);

    actix::spawn(async move {
        cache_pool
            .add_category_send_fail(c, addr.get_ref().clone())
            .await
    });

    Ok(res)
}

pub async fn update_category(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    jwt: UserJwt,
    req: Json<CategoryRequest>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().check_update()?;
    let c = db_pool.admin_update_category(jwt.privilege, req).await?;

    let res = HttpResponse::Ok().json(&c);
    let _ = cache_pool.update_categories(&c).await;

    Ok(res)
}

pub async fn remove_category(
    db_pool: DataRc<MyPostgresPool>,
    // cache_pool: DataRc<MyRedisPool>,
    jwt: UserJwt,
    id: Path<u32>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();

    db_pool.admin_remove_category(id, jwt.privilege).await?;
    //ToDo: fix remove category cache
    //    let _ = cache_pool.remove_category(id).await?;

    Ok(HttpResponse::Ok().finish())
}

pub async fn update_user(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    jwt: UserJwt,
    req: Json<UpdateRequest>,
    addr: DataRc<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().attach_id(None).check_update()?;

    let req = db_pool.update_user_check(jwt.privilege, req).await?;
    let u = db_pool.update_user(req).await?;

    let res = HttpResponse::Ok().json(&u);

    crate::router::user::update_user_send_fail(cache_pool, u, addr);

    Ok(res)
}

pub async fn update_topic(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    jwt: UserJwt,
    req: Json<TopicRequest>,
    addr: DataRc<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().add_user_id(None).check_update()?;

    let t = db_pool.admin_update_topic(jwt.privilege, &req).await?;

    let res = HttpResponse::Ok().json(&t);

    crate::router::topic::update_topic_send_fail(cache_pool, t, addr);

    Ok(res)
}

pub async fn update_post(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    jwt: UserJwt,
    req: Json<PostRequest>,
    addr: DataRc<CacheServiceAddr>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().attach_user_id(None).check_update()?;

    let p = db_pool.admin_update_post(jwt.privilege, req).await?;

    let res = HttpResponse::Ok().json(&p);

    crate::router::post::update_post_send_fail(cache_pool, p, addr);

    Ok(res)
}
