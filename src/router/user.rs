use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, future::{IntoFuture, Either, ok as ft_ok}};

use crate::model::{
    common::Validator,
    user::UpdateRequest,
};
use crate::handler::auth::UserJwt;
use crate::handler::db::DatabaseServiceRaw;
use crate::handler::cache::CacheServiceRaw;

pub fn get(
    jwt: UserJwt,
    db: Data<DatabaseServiceRaw>,
    cache: Data<CacheServiceRaw>,
    req: Path<(u32)>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let id = req.into_inner();

    cache.get_users_from_ids(vec![id])
        .then(move |r| match r {
            Ok(u) => Either::A(
                if id == jwt.user_id {
                    ft_ok(HttpResponse::Ok().json(u.first()))
                } else {
                    ft_ok(HttpResponse::Ok().json(u.first().map(|u| u.to_ref())))
                }
            ),
            Err(_) => Either::B(db
                .get_by_id::<crate::model::user::User>(&db.users_by_id, &vec![id])
                .from_err()
                .and_then(move |u| {
                    let res = if id == jwt.user_id {
                        HttpResponse::Ok().json(u.first())
                    } else {
                        HttpResponse::Ok().json(u.first().map(|u| u.to_ref()))
                    };
                    cache.update_users(u);
                    res
                })
            )
        })
}

pub fn update(
    jwt: UserJwt,
    db: Data<DatabaseServiceRaw>,
    cache: Data<CacheServiceRaw>,
    req: Json<UpdateRequest>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = req.into_inner().attach_id(Some(jwt.user_id));
    req.check_update()
        .into_future()
        .from_err()
        .and_then(move |_| db
            .update_user(req)
            .from_err()
            .and_then(move |u| {
                let res = HttpResponse::Ok().json(&u);
                cache.update_users(vec![u]);
                res
            }))
}