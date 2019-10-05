use actix::prelude::Future as Future01;
use actix_web::{
    web::{Data, Json, Path},
    Error, HttpResponse,
};
use futures::{FutureExt, TryFutureExt};

use crate::handler::cache_update::CacheUpdateAddr;
use crate::handler::{auth::UserJwt, cache::MyRedisPool, db::MyPostgresPool};
use crate::model::{
    category::CategoryRequest,
    common::{GlobalVars, Validator},
    post::PostRequest,
    topic::TopicRequest,
    user::UpdateRequest,
};

pub fn add_category(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    global: Data<GlobalVars>,
    cache: Data<MyRedisPool>,
    db: Data<MyPostgresPool>,
    addr: Data<CacheUpdateAddr>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    add_category_async(jwt, req, global, cache, db, addr)
        .boxed_local()
        .compat()
}

async fn add_category_async(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    global: Data<GlobalVars>,
    cache: Data<MyRedisPool>,
    db: Data<MyPostgresPool>,
    addr: Data<CacheUpdateAddr>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().check_new()?;
    let c = db
        .admin_add_category(jwt.privilege, req, global.get_ref())
        .await?;

    let res = HttpResponse::Ok().json(&c);

    actix::spawn(
        Box::pin(async move { cache.add_category_send_fail(c, addr.into_inner()).await }).compat(),
    );

    Ok(res)
}

pub fn update_category(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    cache: Data<MyRedisPool>,
    db: Data<MyPostgresPool>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    update_category_async(jwt, req, cache, db)
        .boxed_local()
        .compat()
}

async fn update_category_async(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    cache: Data<MyRedisPool>,
    db: Data<MyPostgresPool>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().check_update()?;
    let c = db.admin_update_category(jwt.privilege, req).await?;

    let res = HttpResponse::Ok().json(&c);
    let _ = cache.update_categories(&[c]).await;

    Ok(res)
}

pub fn remove_category(
    jwt: UserJwt,
    id: Path<(u32)>,
    cache: Data<MyRedisPool>,
    db: Data<MyPostgresPool>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    remove_category_async(jwt, id, cache, db)
        .boxed_local()
        .compat()
}

async fn remove_category_async(
    jwt: UserJwt,
    id: Path<(u32)>,
    cache: Data<MyRedisPool>,
    db: Data<MyPostgresPool>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();

    db.admin_remove_category(id, jwt.privilege).await?;
    //ToDo: fix remove category cache
    //    let _ = cache.remove_category(id).await?;

    Ok(HttpResponse::Ok().finish())
}

pub fn update_user(
    jwt: UserJwt,
    req: Json<UpdateRequest>,
    cache: Data<MyRedisPool>,
    db: Data<MyPostgresPool>,
    addr: Data<CacheUpdateAddr>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    update_user_async(jwt, req, cache, db, addr)
        .boxed_local()
        .compat()
}

async fn update_user_async(
    jwt: UserJwt,
    req: Json<UpdateRequest>,
    cache: Data<MyRedisPool>,
    db: Data<MyPostgresPool>,
    addr: Data<CacheUpdateAddr>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().attach_id(None).check_update()?;

    let req = db.update_user_check(jwt.privilege, req).await?;
    let u = db.update_user(req).await?;

    let res = HttpResponse::Ok().json(&u);

    crate::router::user::update_user_send_fail(cache, u, addr);

    Ok(res)
}

pub fn update_topic(
    jwt: UserJwt,
    req: Json<TopicRequest>,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    addr: Data<CacheUpdateAddr>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    update_topic_async(jwt, req, db, cache, addr)
        .boxed_local()
        .compat()
}

async fn update_topic_async(
    jwt: UserJwt,
    req: Json<TopicRequest>,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    addr: Data<CacheUpdateAddr>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().add_user_id(None).check_update()?;

    let t = db.admin_update_topic(jwt.privilege, &req).await?;

    let res = HttpResponse::Ok().json(&t);

    crate::router::topic::update_topic_send_fail(cache, t, addr);

    Ok(res)
}

pub fn update_post(
    jwt: UserJwt,
    req: Json<PostRequest>,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    addr: Data<CacheUpdateAddr>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    update_post_async(jwt, req, db, cache, addr)
        .boxed_local()
        .compat()
}

async fn update_post_async(
    jwt: UserJwt,
    req: Json<PostRequest>,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    addr: Data<CacheUpdateAddr>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().attach_user_id(None).check_update()?;

    let p = db.admin_update_post(jwt.privilege, req).await?;

    let res = HttpResponse::Ok().json(&p);

    crate::router::post::update_post_send_fail(cache, p, addr);

    Ok(res)
}
