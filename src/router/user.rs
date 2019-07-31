use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, future::{IntoFuture, Either, ok as ft_ok}};

use crate::model::{
    actors::{DB, CACHE},
    common::{GlobalVars, Validator},
    user::{AuthRequest, UpdateRequest},
};
use crate::handler::{
    auth::UserJwt,
    messenger::AddActivationMail,
    cache::{UpdateCache, ActivateUser, DeleteCache},
    user::{Login, Register, UpdateUser, GetUsers, GetUsersCache},
};

pub fn get(
    jwt: UserJwt,
    db: Data<DB>,
    cache: Data<CACHE>,
    req: Path<(u32)>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let id = req.into_inner();

    cache.send(GetUsersCache(vec![id]))
        .from_err()
        .and_then(move |r| match r {
            Ok(u) => Either::A(
                if id == jwt.user_id {
                    ft_ok(HttpResponse::Ok().json(u.first()))
                } else {
                    ft_ok(HttpResponse::Ok().json(u.first().map(|u| u.to_ref())))
                }
            ),
            Err(_) => Either::B(
                db.send(GetUsers(vec![id]))
                    .from_err()
                    .and_then(|r| r)
                    .from_err()
                    .and_then(move |u| {
                        let res = if id == jwt.user_id {
                            HttpResponse::Ok().json(u.first())
                        } else {
                            HttpResponse::Ok().json(u.first().map(|u| u.to_ref()))
                        };
                        let _ = cache.do_send(UpdateCache::User(u));
                        res
                    })
            )
        })
}

pub fn update(
    jwt: UserJwt,
    db: Data<DB>,
    cache: Data<CACHE>,
    req: Json<UpdateRequest>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = req.into_inner().attach_id(Some(jwt.user_id));
    req.check_update()
        .into_future()
        .from_err()
        .and_then(move |_| db
            .send(UpdateUser(req))
            .from_err()
            .and_then(|r| r)
            .from_err()
            .and_then(move |u| {
                let res = HttpResponse::Ok().json(&u);
                let _ = cache.do_send(UpdateCache::User(vec![u]));
                res
            }))
}

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

pub fn activation(
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