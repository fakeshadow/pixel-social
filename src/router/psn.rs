use std::ops::Deref;

use actix_web::{
    Error,
    HttpResponse, web::{Data, Json, Query},
};
use futures::{
    FutureExt,
    TryFutureExt,
};
use futures01::{
    future::{Either, IntoFuture, ok as ft_ok},
    Future as Future01,
};

use crate::{
    handler::{
        auth::{
            UserJwt,
            UserJwtOpt,
        },
        cache::CacheService,
        db::DatabaseService,
    },
    model::{
        errors::ResError,
        psn::PSNRequest,
    },
};

pub fn query_handler(
    req: Query<PSNRequest>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future01<Item=HttpResponse, Error=Error> {
    query_handler_async(req, db, cache).boxed_local().compat().from_err()
}

async fn query_handler_async(
    req: Query<PSNRequest>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> Result<HttpResponse, ResError> {
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
                if let Ok(s) = db.get_trophy_set(p.np_id.as_str(), np_communication_id.as_str()).await {
                    return Ok(HttpResponse::Ok().json(&s));
                }
            }
        }
        _ => (),
    };

    let req = req.stringify()?;
    let _ = cache.add_psn_request_now(req.as_str()).await?;

    Ok(HttpResponse::Ok().finish())
}

pub fn query_handler_with_jwt(
    jwt: UserJwt,
    req: Query<PSNRequest>,
    cache: Data<CacheService>,
) -> impl Future01<Item=HttpResponse, Error=Error> {
    query_handler_with_jwt_async(jwt, req, cache).boxed_local().compat().from_err()
}

async fn query_handler_with_jwt_async(
    jwt: UserJwt,
    req: Query<PSNRequest>,
    cache: Data<CacheService>,
) -> Result<HttpResponse, ResError> {
    match req.deref() {
        PSNRequest::Auth { .. } => {
            let req = req
                .into_inner()
                .check_privilege(jwt.privilege)?
                .stringify()?;

            let _ = cache.add_psn_request_privilege(req.as_str()).await?;
        }
        PSNRequest::Activation { .. } => {
            let req = req
                .into_inner()
                .attach_user_id(jwt.user_id)
                .stringify()?;

            let _ = cache.add_psn_request_now(req.as_str()).await?;
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
) -> impl Future01<Item=HttpResponse, Error=Error> {
    let jwt_opt = jwt_opt.0;

    ft_ok(HttpResponse::Ok().json("you are good"))
}
