use actix_web::{
    Error,
    HttpResponse, web::{Data, Json, Path},
};
use futures::{FutureExt, TryFutureExt};
use futures01::Future as Future01;

use crate::handler::{auth::UserJwt, cache::CacheService, db::DatabaseService};
use crate::handler::cache::{AddToCache, CheckCacheConn};
use crate::model::{
    category::CategoryRequest,
    common::{GlobalVars, Validator},
    errors::ResError,
    post::PostRequest,
    topic::TopicRequest,
    user::UpdateRequest,
};

pub fn add_category(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    global: Data<GlobalVars>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> impl Future01<Item=HttpResponse, Error=Error> {
    add_category_async(jwt, req, global, cache, db)
        .boxed_local()
        .compat()
        .from_err()
}

async fn add_category_async(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    global: Data<GlobalVars>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> Result<HttpResponse, ResError> {
    let req = req.into_inner().check_new()?;
    let c = db
        .admin_add_category(jwt.privilege, req, global.get_ref())
        .await?;

    let res = HttpResponse::Ok().json(&c);

    match cache.check_cache_conn().await {
        Ok(opt) => {
            actix::spawn(
                cache
                    .if_replace_cache(opt)
                    .add_category_cache_01(&c)
                    .map_err(move |_| cache.send_failed_category(c)),
            );
        }
        Err(_) => cache.send_failed_category(c),
    };

    Ok(res)
}

pub fn update_category(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> impl Future01<Item=HttpResponse, Error=Error> {
    update_category_async(jwt, req, cache, db)
        .boxed_local()
        .compat()
        .from_err()
}

async fn update_category_async(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> Result<HttpResponse, ResError> {
    let req = req.into_inner().check_update()?;
    let c = db.admin_update_category(jwt.privilege, req).await?;

    let res = HttpResponse::Ok().json(&c);
    cache.update_categories(&[c]);

    Ok(res)
}

pub fn remove_category(
    jwt: UserJwt,
    id: Path<(u32)>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> impl Future01<Item=HttpResponse, Error=Error> {
    remove_category_async(jwt, id, cache, db)
        .boxed_local()
        .compat()
        .from_err()
}

async fn remove_category_async(
    jwt: UserJwt,
    id: Path<(u32)>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> Result<HttpResponse, ResError> {
    let id = id.into_inner();

    let _ = db.admin_remove_category(id, jwt.privilege).await?;
    //ToDo: fix remove category cache
    //    let _ = cache.remove_category(id).await?;

    Ok(HttpResponse::Ok().finish())
}

pub fn update_user(
    jwt: UserJwt,
    req: Json<UpdateRequest>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> impl Future01<Item=HttpResponse, Error=Error> {
    update_user_async(jwt, req, cache, db).boxed_local().compat()
}

async fn update_user_async(
    jwt: UserJwt,
    req: Json<UpdateRequest>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> Result<HttpResponse, Error> {
    let req = req
        .into_inner()
        .attach_id(None)
        .check_update()?;

    let req = db.update_user_check(jwt.privilege, req).await?;

    let u = db.check_conn().await?.update_user(req).await?;

    let res = HttpResponse::Ok().json(&u);

    crate::router::user::update_user_with_fail_check(cache, u).await;

    Ok(res)
}

pub fn update_topic(
    jwt: UserJwt,
    req: Json<TopicRequest>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> impl Future01<Item=HttpResponse, Error=Error> {
    update_topic_async(jwt, req, cache, db).boxed_local().compat()
}

async fn update_topic_async(
    jwt: UserJwt,
    req: Json<TopicRequest>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner()
        .attach_user_id(None)
        .check_update()?;

    let t = db
        .check_conn()
        .await?
        .admin_update_topic(jwt.privilege, &req)
        .await?;

    let res = HttpResponse::Ok().json(&t);

    crate::router::topic::update_topic_with_fail_check(cache, t).await;

    Ok(res)
}

pub fn update_post(
    jwt: UserJwt,
    req: Json<PostRequest>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future01<Item=HttpResponse, Error=Error> {
    update_post_async(jwt, req, db, cache).boxed_local().compat()
}

async fn update_post_async(
    jwt: UserJwt,
    req: Json<PostRequest>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner()
        .attach_user_id(None)
        .check_update()?;

    let p = db.check_conn().await?.admin_update_post(jwt.privilege, req).await?;

    let res = HttpResponse::Ok().json(&p);

    crate::router::post::update_post_with_fail_check(cache, p).await;

    Ok(res)
}
