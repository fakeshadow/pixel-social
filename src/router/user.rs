use actix_web::{
    web::{Data, Json, Path},
    Error, HttpResponse,
};
use futures::{
    future::{ok as ft_ok, Either, IntoFuture},
    Future,
};

use crate::handler::{auth::UserJwt, cache::CacheService, db::DatabaseService};
use crate::model::{common::Validator, user::UpdateRequest};

pub fn get(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Path<(u32)>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let id = req.into_inner();

    cache.get_users_from_ids(vec![id]).then(move |r| match r {
        Ok(u) => Either::A(if id == jwt.user_id {
            ft_ok(HttpResponse::Ok().json(u.first()))
        } else {
            ft_ok(HttpResponse::Ok().json(u.first().map(|u| u.to_user_ref())))
        }),
        Err(_) => Either::B(db.get_users_by_id(&[id]).from_err().and_then(move |u| {
            let res = if id == jwt.user_id {
                HttpResponse::Ok().json(u.first())
            } else {
                HttpResponse::Ok().json(u.first().map(|u| u.to_user_ref()))
            };
            cache.update_users(&u);
            res
        })),
    })
}

pub fn update(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Json<UpdateRequest>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    req.into_inner()
        .attach_id(Some(jwt.user_id))
        .check_update()
        .into_future()
        .from_err()
        .and_then(move |req| {
            db.check_conn()
                .and_then(move |opt| db.if_replace_db(opt).update_user(req))
        })
        .from_err()
        .and_then(move |u| {
            let res = HttpResponse::Ok().json(&u);
            cache.update_users(&[u]);
            res
        })
}
