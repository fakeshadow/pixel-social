use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, future::{Either, ok as ft_ok}};

use crate::handler::{auth::UserJwt, cache::UpdateCacheAsync};
use crate::model::{
    common::{GlobalGuard, PostgresPool, RedisPool, Response},
    user::{ToUserRef, AuthRequest, UpdateRequest},
};

pub fn get_user(
    jwt: UserJwt,
    id: Path<u32>,
    db: Data<PostgresPool>,
    cache: Data<RedisPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    use crate::model::{user::IdToQuery, cache::IdToUserQuery};
    id.to_query_cache()
        .into_user(cache.get_ref())
        .then(move |res| match res {
            Ok(u) => Either::A(if u.id == jwt.user_id {
                ft_ok(HttpResponse::Ok().json(u))
            } else {
                ft_ok(HttpResponse::Ok().json(u.to_ref()))
            }),
            Err(_) => Either::B(
                id.to_query()
                    .into_user(db.get_ref().clone(), None)
                    .from_err()
                    .and_then(move |u| {
                        let res = if u.id == jwt.user_id {
                            HttpResponse::Ok().json(&u)
                        } else {
                            HttpResponse::Ok().json(u.to_ref())
                        };
                        UpdateCacheAsync::GotUser(u).handler(cache.get_ref()).then(|_| res)
                    })
            )
        })
}

pub fn register_user(
    req: Json<AuthRequest>,
    global: Data<GlobalGuard>,
    db: Data<PostgresPool>,
    cache: Data<RedisPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .into_register_query()
        .into_user(db.get_ref().clone(), Some(global.get_ref().clone()))
        .from_err()
        .and_then(move |u| UpdateCacheAsync::GotUser(u)
            .handler(&cache)
            .then(|_| Response::Registered.to_res()))
}

pub fn update_user(
    jwt: UserJwt,
    req: Json<UpdateRequest>,
    db: Data<PostgresPool>,
    cache: Data<RedisPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .attach_id_into(Some(jwt.user_id))
        .into_update_query()
        .into_user(db.get_ref().clone(), None)
        .from_err()
        .and_then(move |u| {
            let res = HttpResponse::Ok().json(u.to_ref());
            UpdateCacheAsync::GotUser(u).handler(&cache).then(|_| res)
        })
}

pub fn login_user(
    req: Json<AuthRequest>,
    db: Data<PostgresPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .into_login_query()
        .into_jwt_user(db.get_ref().clone())
        .from_err()
        .and_then(|u| HttpResponse::Ok().json(&u))
}