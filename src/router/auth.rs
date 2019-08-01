use actix_web::{HttpResponse, Error, web::{Data, Json, Path}, ResponseError};
use futures::{Future, future::{IntoFuture, Either, ok as ft_ok}};

use crate::model::{
    actors::{DB, CACHE},
    errors::ResError,
    common::{GlobalVars, Validator},
    user::{AuthRequest, UpdateRequest, User},
};
use crate::handler::{
    messenger::AddActivationMail,
    cache::{UpdateCache, DeleteCache},
    auth::{UserJwt, Login, Register, ActivateUser},
    user::{GetUsers, GetUsersCache, UpdateUser},
};

pub fn login(
    db: Data<DB>,
    req: Json<AuthRequest>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    req.check_login()
        .into_future()
        .from_err()
        .and_then(move |_| db
            .send(Login(req.into_inner()))
            .from_err()
            .and_then(|r| r)
            .from_err()
            .and_then(|t| HttpResponse::Ok().json(&t)))
}

pub fn register(
    db: Data<DB>,
    cache: Data<CACHE>,
    global: Data<GlobalVars>,
    req: Json<AuthRequest>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    req.check_register()
        .into_future()
        .from_err()
        .and_then(move |_| db
            .send(Register(req.into_inner(), global.get_ref().clone()))
            .from_err()
            .and_then(|r| r)
            .from_err()
            .and_then(move |u| {
                let res = HttpResponse::Ok().json(&u);
                let _ = cache.do_send(AddActivationMail(u.clone()));
                let _ = cache.do_send(UpdateCache::User(vec![u]));
                res
            })
        )
}

pub fn activate_by_mail(
    db: Data<DB>,
    cache: Data<CACHE>,
    req: Path<(String)>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let uuid = req.into_inner();

    cache.send(ActivateUser(uuid.clone()))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(move |uid| db
            .send(UpdateUser(UpdateRequest::make_active(uid)))
            .from_err()
            .and_then(|r| r)
            .from_err()
            .and_then(move |u| {
                //ToDo: sign a new jwt token and return auth response instead of user object.
                let res = HttpResponse::Ok().json(&u);
                let _ = cache.do_send(UpdateCache::User(vec![u]));
                let _ = cache.do_send(DeleteCache::Mail(uuid));
                res
            }))
}

pub fn add_activation_mail(
    jwt: UserJwt,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    cache.send(GetUsersCache(vec![jwt.user_id]))
        .from_err()
        .and_then(move |r| match r {
            Ok(u) => Either::A(ft_ok(pop_user_add_activation_mail(cache, u))),
            Err(e) => Either::B(match e {
                ResError::IdsFromCache(ids) => Either::A(db
                    .send(GetUsers(ids))
                    .from_err()
                    .and_then(|r| r)
                    .from_err()
                    .and_then(|u| pop_user_add_activation_mail(cache, u))),
                _ => Either::B(ft_ok(e.render_response()))
            })
        })
}

fn pop_user_add_activation_mail(cache: Data<CACHE>, mut u: Vec<User>) -> HttpResponse {
    match u.pop() {
        Some(u) => {
            let _ = cache.do_send(AddActivationMail(u));
            HttpResponse::Ok().finish()
        }
        None => ResError::BadRequest.render_response()
    }
}