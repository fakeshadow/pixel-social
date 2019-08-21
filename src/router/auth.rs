use actix_web::{
    web::{Data, Json, Path},
    Error, HttpResponse, ResponseError,
};
use futures::{
    future::{ok as ft_ok, Either, IntoFuture},
    Future,
};

use crate::handler::{auth::UserJwt, cache::CacheService, db::DatabaseService};
use crate::model::{
    common::{GlobalVars, Validator},
    errors::ResError,
    user::{AuthRequest, UpdateRequest, User},
};

pub fn login(
    db: Data<DatabaseService>,
    req: Json<AuthRequest>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    req.check_login()
        .into_future()
        .from_err()
        .and_then(move |_| {
            db.login(req.into_inner())
                .from_err()
                .and_then(|t| HttpResponse::Ok().json(&t))
        })
}

pub fn register(
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    global: Data<GlobalVars>,
    req: Json<AuthRequest>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    req.check_register()
        .into_future()
        .from_err()
        .and_then(move |_| {
            db.check_register(req.into_inner())
                .from_err()
                .and_then(move |req| {
                    db.register(req, global.get_ref())
                        .from_err()
                        .and_then(move |u| {
                            let res = HttpResponse::Ok().json(&u);
                            cache.add_activation_mail(u.clone());
                            cache.update_users(&[u]);
                            res
                        })
                })
        })
}

pub fn activate_by_mail(
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Path<(String)>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let uuid = req.into_inner();

    cache
        .get_uid_from_uuid(uuid.as_str())
        .from_err()
        .and_then(move |uid| {
            db.update_user(UpdateRequest::make_active(uid))
                .from_err()
                .and_then(move |u| {
                    // request for another login after the activation to update user's jwt token.
                    let res = HttpResponse::Ok().json(&u);
                    cache.update_users(&[u]);
                    cache.remove_activation_uuid(uuid.as_str());
                    res
                })
        })
}

pub fn add_activation_mail(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    cache
        .get_users_from_ids(vec![jwt.user_id])
        .then(move |r| match r {
            Ok(u) => Either::A(ft_ok(pop_user_add_activation_mail(cache, u))),
            Err(e) => Either::B(match e {
                ResError::IdsFromCache(ids) => Either::A(
                    db.get_by_id(&db.users_by_id, &ids)
                        .from_err()
                        .and_then(|u| pop_user_add_activation_mail(cache, u)),
                ),
                _ => Either::B(ft_ok(e.render_response())),
            }),
        })
}

fn pop_user_add_activation_mail(cache: Data<CacheService>, mut u: Vec<User>) -> HttpResponse {
    match u.pop() {
        Some(u) => {
            cache.add_activation_mail(u);
            HttpResponse::Ok().finish()
        }
        None => ResError::BadRequest.render_response(),
    }
}
