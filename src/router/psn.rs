use actix_web::{web::{Data, Form}, Error, HttpResponse, ResponseError};
use futures::{
    future::{ok as ft_ok, Either, IntoFuture},
    Future,
};

use crate::{
    model::psn::PSNActivationRequest,
    handler::{auth::UserJwt, cache::CacheService},
};
use crate::model::psn::{PSNRequest, PSNProfileRequest};


pub fn register(
    jwt: UserJwt,
    cache: Data<CacheService>,
    mut req: Form<PSNActivationRequest>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .attach_user_id(jwt.user_id)
        .into_request_string()
        .into_future()
        .from_err()
        .and_then(move |req|
            cache.add_psn_request(req.as_str())
                .from_err()
                .and_then(|_| HttpResponse::Ok().finish())
        )
}

pub fn get_profile(
    cache: Data<CacheService>,
    req: Form<PSNProfileRequest>,
) -> impl Future<Item=HttpResponse, Error=Error> {

    req.into_inner()
        .into_request_string()
        .into_future()
        .from_err()
        .and_then(move |req|
            cache.add_psn_request(req.as_str())
                .from_err()
                .and_then(|_| HttpResponse::Ok().finish())
        )
//    let req = req.into_inner();
//    cache.get_psn_profile(req.0)
//        .then(|r| match r {
//            Ok(r) => Either::A(HttpResponse::Ok(r).json()),
//            Err(_) => Either::B(
//                req.into_inner()
//                    .into_request_string()
//                    .into_future()
//                    .from_err()
//                    .and_then(move |req|
//                        cache.add_psn_request(req.as_str())
//                            .from_err()
//                            .and_then(|_| HttpResponse::Ok().finish())
//                    )
//            )
//        })
}