use actix_web::{
    Error,
    HttpResponse, ResponseError, web::{Data, Form},
};
use futures::{
    future::{Either, IntoFuture, ok as ft_ok},
    Future,
};
use serde::Serialize;

use crate::{
    handler::{
        auth::UserJwt,
        cache::CacheService,
        db::DatabaseService,
    },
    model::psn::{
        PSNActivationRequest,
        PSNProfileRequest,
        Stringify,
    },
};

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
        .and_then(move |s|
            cache.add_psn_request(s.as_str())
                .from_err()
                .and_then(|_| HttpResponse::Ok().finish())
        )
}

pub fn profile(
    cache: Data<CacheService>,
    req: Form<PSNProfileRequest>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = req.into_inner();

    cache.get_psn_profile(req.online_id.as_bytes())
        .then(move |r| match r {
            Ok(u) => Either::A(ft_ok(HttpResponse::Ok().json(&u))),
            Err(_) => Either::B(
                req.stringify()
                    .into_future()
                    .from_err()
                    .and_then(move |s|
                        cache.add_psn_request(s.as_str())
                            .from_err()
                            .and_then(|_| HttpResponse::Ok().finish())
                    )
            )
        })
}
