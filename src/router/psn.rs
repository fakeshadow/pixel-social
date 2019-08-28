use std::ops::Deref;

use actix_web::{
    web::{Data, Json, Query},
    Error, HttpResponse,
};
use futures::{FutureExt, TryFutureExt};
use futures01::Future as Future01;

use crate::{
    handler::{
        auth::{UserJwt, UserJwtOpt},
        cache::CacheService,
        db::DatabaseService,
        psn::{AddPSNRequest, PSNServiceAddr},
    },
    model::psn::PSNRequest,
};

pub fn query_handler(
    req: Query<PSNRequest>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    addr: Data<PSNServiceAddr>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    query_handler_async(req, db, cache, addr)
        .boxed_local()
        .compat()
}

async fn query_handler_async(
    req: Query<PSNRequest>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    addr: Data<PSNServiceAddr>,
) -> Result<HttpResponse, Error> {
    match req.deref() {
        PSNRequest::Profile { online_id } => {
            if let Ok(p) = cache.get_psn_profile(online_id.as_str()).await {
                return Ok(HttpResponse::Ok().json(&p));
            }
        }
        PSNRequest::TrophyTitles { online_id, page } => {
            let page = page.parse::<u32>().unwrap_or(1);

            if let Ok(p) = cache.get_psn_profile(online_id.as_str()).await {
                if let Ok(t) = db.get_trophy_titles(p.np_id.as_str(), page).await {
                    return Ok(HttpResponse::Ok().json(&t));
                }
            }
        }
        PSNRequest::TrophySet {
            online_id,
            np_communication_id,
        } => {
            if let Ok(p) = cache.get_psn_profile(online_id.as_str()).await {
                if let Ok(s) = db
                    .get_trophy_set(p.np_id.as_str(), np_communication_id.as_str())
                    .await
                {
                    return Ok(HttpResponse::Ok().json(&s));
                }
            }
        }
        _ => (),
    };

    addr.do_send(AddPSNRequest(req.into_inner(), false));

    Ok(HttpResponse::Ok().finish())
}

pub fn query_handler_with_jwt(
    jwt: UserJwt,
    req: Query<PSNRequest>,
    addr: Data<PSNServiceAddr>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    query_handler_with_jwt_async(jwt, req, addr)
        .boxed_local()
        .compat()
}

async fn query_handler_with_jwt_async(
    jwt: UserJwt,
    req: Query<PSNRequest>,
    addr: Data<PSNServiceAddr>,
) -> Result<HttpResponse, Error> {
    match req.deref() {
        PSNRequest::Auth { .. } => {
            let req = req.into_inner().check_privilege(jwt.privilege)?;

            addr.do_send(AddPSNRequest(req, true));
        }
        PSNRequest::Activation { .. } => {
            addr.do_send(AddPSNRequest(
                req.into_inner().attach_user_id(jwt.user_id),
                false,
            ));
        }
        _ => (),
    };
    Ok(HttpResponse::Ok().finish())
}

pub fn community(
    jwt_opt: UserJwtOpt,
    //    req: Json<>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    community_async(jwt_opt, db, cache).boxed_local().compat()
}

async fn community_async(
    jwt_opt: UserJwtOpt,
    //    req: Json<>,
    _db: Data<DatabaseService>,
    _cache: Data<CacheService>,
) -> Result<HttpResponse, Error> {
    let _jwt_opt = jwt_opt.0;
    Ok(HttpResponse::Ok().finish())
}
