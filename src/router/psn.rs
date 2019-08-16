use std::ops::Deref;

use actix_web::{
    Error,
    HttpResponse, web::{Data, Form},
};
use futures::{
    future::{Either, IntoFuture, ok as ft_ok},
    Future,
};

use crate::{
    handler::{
        auth::UserJwt,
        cache::CacheService,
        db::DatabaseService,
    },
    model::{
        errors::ResError,
        psn::{
            PSNActivationRequest,
            PSNAuthRequest,
            PSNProfileRequest,
            PSNTrophyRequest,
            Stringify,
        },
    },
};

pub fn auth(
    jwt: UserJwt,
    req: Form<PSNAuthRequest>,
    cache: Data<CacheService>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .check_privilege(jwt.privilege)
        .into_future()
        .from_err()
        .and_then(|req| req.stringify())
        .from_err()
        .and_then(move |s| {
            cache
                .add_psn_request_with_privilege(s.as_str())
                .from_err()
                .and_then(|_| HttpResponse::Ok().finish())
        })
}

pub fn register(
    jwt: UserJwt,
    cache: Data<CacheService>,
    req: Form<PSNActivationRequest>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .attach_user_id(jwt.user_id)
        .stringify()
        .into_future()
        .from_err()
        .and_then(move |s| {
            cache
                .add_psn_request_now(s.as_str())
                .from_err()
                .and_then(|_| HttpResponse::Ok().finish())
        })
}

// psn profile only stores in cache. as the latest data are always from the psn.
pub fn profile(
    cache: Data<CacheService>,
    req: Form<PSNProfileRequest>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    cache
        .get_psn_profile(req.deref().online_id.as_bytes())
        .then(|r| handle_response(r, req.into_inner(), cache))
}

// trophy data only stores in database as the list are big and not frequent query.
// include np_communication_id=NPWRXXXX result in user trophy set query.
pub fn trophy(
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Form<PSNTrophyRequest>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    match req.np_communication_id.as_ref() {
        Some(_) => Either::A(
            db.get_trophy_set(req.deref())
                .then(|r| handle_response(r, req.into_inner(), cache))
        ),
        None => Either::B(
            db.get_trophy_titles(req.np_communication_id.as_ref().unwrap(), 1)
                .then(|r| handle_response(r, req.into_inner(), cache))
        )
    }
}

fn handle_response<T, E>(
    r: Result<T, ResError>,
    req: E,
    cache: Data<CacheService>,
) -> impl Future<Item=HttpResponse, Error=Error>
    where
        T: serde::Serialize,
        E: Stringify,
{
    match r {
        Ok(u) => Either::A(ft_ok(HttpResponse::Ok().json(&u))),
        Err(_) => Either::B(req.stringify().into_future().from_err().and_then(move |s| {
            cache
                .add_psn_request_now(s.as_str())
                .from_err()
                .and_then(|_| HttpResponse::Ok().finish())
        })),
    }
}
