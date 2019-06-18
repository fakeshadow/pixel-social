use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, future::{IntoFuture, Either, ok as ft_ok}};

use crate::model::{
    actors::{DB, CACHE},
    common::{GlobalGuard, Validator},
    user::{AuthRequest, UpdateRequest, ToUserRef},
};
use crate::handler::{
    auth::UserJwt,
    cache::{UpdateCache, GetUsersCache},
    user::{Login, PreRegister, Register, UpdateUser, GetUsers},
};

pub fn get(
    jwt: UserJwt,
    db: Data<DB>,
    cache: Data<CACHE>,
    req: Path<(u32)>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let id = req.into_inner();
    cache.send(GetUsersCache(vec![id])).
        from_err()
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
                let _ = cache.do_send(UpdateCache::User(u));
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
    global: Data<GlobalGuard>,
    req: Json<AuthRequest>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    req.check_register()
        .into_future()
        .from_err()
        .and_then(move |_| db
            .send(PreRegister(req.into_inner()))
            .from_err()
            .and_then(|r| r)
            .from_err()
            .and_then(move |req| db
                .send(Register(req, global.get_ref().clone()))
                .from_err()
                .and_then(|r| r)
                .from_err()
                .and_then(move |u| {
                    let res = HttpResponse::Ok().json(&u);
                    let _ = cache.do_send(UpdateCache::User(u));
                    res
                }))
        )
}