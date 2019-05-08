use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, future::{Either, ok as ft_ok}};

use crate::handler::{
    auth::UserJwt,
    cache::UpdateCache,
    user::UserQuery};
use crate::model::{
    errors::ServiceError,
    common::{GlobalGuard, PostgresPool, QueryOption, QueryOptAsync, RedisPool},
    user::{ToUserRef, AuthRequest, UserUpdateRequest},
};

pub fn get_user(jwt: UserJwt, id: Path<u32>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                      -> impl Future<Item=HttpResponse, Error=Error> {
    use crate::model::{user::IdToQuery, cache::IdToUserQueryAsync};
    id.into_query_cache()
        .user_from_cache(cache.clone())
        .then(move |res| match res {
            Ok(u) => Either::A(if u.id == jwt.user_id {
                ft_ok(HttpResponse::Ok().json(u))
            } else {
                ft_ok(HttpResponse::Ok().json(u.to_ref()))
            }),
            Err(_) => Either::B(
                id.into_query()
                    .into_user(QueryOptAsync::new(Some(db), None))
                    .from_err()
                    .and_then(move |u| {
                        let _ignore = UpdateCache::GotUser(&u).handle_update(&Some(&cache));
                        if u.id == jwt.user_id {
                            HttpResponse::Ok().json(u)
                        } else {
                            HttpResponse::Ok().json(u.to_ref())
                        }
                    })
            )
        })
}

pub fn register_user(req: Json<AuthRequest>, global: Data<GlobalGuard>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                           -> impl Future<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .into_register_query()
        .into_user(QueryOptAsync::new(Some(db), Some(global)))
        .from_err()
        .and_then(move |u| {
            let _ignore = UpdateCache::GotUser(&u).handle_update(&Some(&cache));
            HttpResponse::Ok().json(u.to_ref())
        })
}

pub fn update_user(jwt: UserJwt, req: Json<UserUpdateRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                         -> impl Future<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .attach_id(Some(jwt.user_id))
        .into_update_query()
        .into_user(QueryOptAsync::new(Some(db), None))
        .from_err()
        .and_then(move |u| {
            let _ignore = UpdateCache::GotUser(&u).handle_update(&Some(&cache));
            HttpResponse::Ok().json(u.to_ref())
        })
}

pub fn login_user(req: Json<AuthRequest>, db: Data<PostgresPool>)
                        -> impl Future<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .into_login_query()
        .into_login(QueryOptAsync::new(Some(db), None))
        .from_err()
        .and_then(|u| HttpResponse::Ok().json(&u))
}