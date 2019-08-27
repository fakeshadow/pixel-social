use std::future::Future;

use actix_web::{
    Error,
    HttpResponse, web::{Data, Json, Path},
};
use futures::{
    FutureExt,
    TryFutureExt,
};
use futures01::Future as Future01;

use crate::handler::{
    auth::UserJwt,
    cache::CacheService,
    db::DatabaseService,
};
use crate::model::{
    common::Validator,
    errors::ResError,
    user::UpdateRequest,
};

pub fn get(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Path<(u32)>,
) -> impl Future01<Item=HttpResponse, Error=Error> {
    get_async(jwt, db, cache, req).boxed_local().compat().from_err()
}

async fn get_async(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Path<(u32)>,
) -> Result<HttpResponse, ResError> {
    let id = req.into_inner();
    let u = match cache.get_users_from_ids(vec![id]).await {
        Ok(u) => u,
        Err(_) => db.get_users_by_id(&[id]).await?
    };

    if id == jwt.user_id {
        Ok(HttpResponse::Ok().json(u.first()))
    } else {
        Ok(HttpResponse::Ok().json(u.first().map(|u| u.to_user_ref())))
    }
}

pub fn update(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Json<UpdateRequest>,
) -> impl Future01<Item=HttpResponse, Error=Error> {
    update_async(jwt, db, cache, req).boxed_local().compat().from_err()
}

async fn update_async(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Json<UpdateRequest>,
) -> Result<HttpResponse, Error> {
    let req = req
        .into_inner()
        .attach_id(Some(jwt.user_id))
        .check_update()?;

    let opt = db.check_conn().await?;

    let u = db.if_replace_db(opt).update_user(req).await?;

    let res = HttpResponse::Ok().json(&u);

    actix::spawn(
        cache.update_user_return_fail(vec![u])
            .map_err(move |u| cache.send_failed_user(u))
    );

    Ok(res)
}
