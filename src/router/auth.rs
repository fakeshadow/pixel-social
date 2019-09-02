use actix_web::{
    web::{Data, Json, Path},
    Error, HttpResponse,
};
use futures::{FutureExt, TryFutureExt};
use futures01::Future as Future01;

use crate::handler::{auth::UserJwt, cache::CacheService, db::DatabaseService};
use crate::model::{
    common::{GlobalVars, Validator},
    errors::ResError,
    user::{AuthRequest, UpdateRequest},
};

pub fn login(
    db: Data<DatabaseService>,
    req: Json<AuthRequest>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    login_async(db, req).boxed_local().compat()
}

async fn login_async(
    db: Data<DatabaseService>,
    req: Json<AuthRequest>,
) -> Result<HttpResponse, Error> {
    let r = req.into_inner().check_login()?;
    let r = db.login(r).await?;
    Ok(HttpResponse::Ok().json(&r))
}

pub fn register(
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    global: Data<GlobalVars>,
    req: Json<AuthRequest>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    register_async(db, cache, global, req)
        .boxed_local()
        .compat()
}

async fn register_async(
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    global: Data<GlobalVars>,
    req: Json<AuthRequest>,
) -> Result<HttpResponse, Error> {
    let req = req.into_inner().check_register()?;

    let u = db
        .check_conn()
        .await?
        .register(req, global.get_ref())
        .await?;

    let res = HttpResponse::Ok().json(&u);

    cache.add_activation_mail(u.clone());

    crate::router::user::update_user_with_fail_check(cache, u);

    Ok(res)
}

pub fn activate_by_mail(
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Path<(String)>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    activate_by_mail_async(db, cache, req)
        .boxed_local()
        .compat()
}

async fn activate_by_mail_async(
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Path<(String)>,
) -> Result<HttpResponse, Error> {
    let uuid = req.into_inner();

    let uid = cache.get_uid_from_uuid(uuid.as_str()).await?;

    let u = db.update_user(UpdateRequest::make_active(uid)).await?;

    let res = HttpResponse::Ok().json(&u);

    cache.remove_activation_uuid(uuid.as_str());

    crate::router::user::update_user_with_fail_check(cache, u);

    Ok(res)
}

pub fn add_activation_mail(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    add_activation_mail_async(jwt, db, cache)
        .boxed_local()
        .compat()
}

async fn add_activation_mail_async(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> Result<HttpResponse, Error> {
    let mut u = match cache.get_users_from_ids(vec![jwt.user_id]).await {
        Ok(u) => u,
        Err(e) => {
            if let ResError::IdsFromCache(ids) = e {
                db.get_users_by_id(&ids).await?
            } else {
                return Err(e.into());
            }
        }
    };

    match u.pop() {
        Some(u) => {
            cache.add_activation_mail(u);
            Ok(HttpResponse::Ok().finish())
        }
        None => Err(ResError::BadRequest.into()),
    }
}
