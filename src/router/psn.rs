use std::ops::Deref;

use actix_web::{
    web::{Data, Query},
    Error, HttpResponse,
};
use futures::{
    future::{ok as ft_ok, Either, IntoFuture},
    Future,
};

use crate::{
    handler::{auth::UserJwt, cache::CacheService, db::DatabaseService},
    model::{errors::ResError, psn::PSNRequest},
};

pub fn query_handler(
    req: Query<PSNRequest>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    match req.deref() {
        PSNRequest::Profile { online_id } => Either::A(Either::A(
            cache
                .get_psn_profile(online_id.as_str())
                .then(|r| handle_response(r, req.into_inner(), cache)),
        )),
        PSNRequest::TrophyTitles { online_id, page } => Either::A(Either::B({
            let page = page.parse::<u32>().unwrap_or(1);
            cache
                .get_psn_profile(online_id.as_str())
                .from_err()
                .and_then(move |u| {
                    db.get_trophy_titles(u.np_id.as_str(), page)
                        .then(|r| handle_response(r, req.into_inner(), cache))
                })
        })),
        PSNRequest::TrophySet {
            online_id,
            np_communication_id,
        } => Either::B(Either::A({
            let np_communication_id = np_communication_id.to_owned();
            cache
                .get_psn_profile(online_id.as_str())
                .from_err()
                .and_then(move |u| {
                    db.get_trophy_set(u.np_id.as_str(), np_communication_id.as_str())
                        .then(|r| handle_response(r, req.into_inner(), cache))
                })
        })),
        _ => Either::B(Either::B(ft_ok(HttpResponse::Ok().finish()))),
    }
}

pub fn query_handler_with_jwt(
    jwt: UserJwt,
    req: Query<PSNRequest>,
    cache: Data<CacheService>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    match req.deref() {
        PSNRequest::Auth {
            uuid: _,
            two_step: _,
            refresh_token: _,
        } => Either::A(Either::A(
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
                }),
        )),
        PSNRequest::Activation {
            user_id: _,
            online_id: _,
            code: _,
        } => Either::A(Either::B(
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
                }),
        )),
        _ => Either::B(ft_ok(HttpResponse::Ok().finish())),
    }
}

fn handle_response<T: serde::Serialize>(
    r: Result<T, ResError>,
    req: PSNRequest,
    cache: Data<CacheService>,
) -> impl Future<Item = HttpResponse, Error = Error> {
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
