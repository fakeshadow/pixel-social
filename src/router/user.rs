use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, IntoFuture, future::{Either, ok as ft_ok}};

use crate::handler::{auth::UserJwt, cache::handle_cache_query, user_async::UserQueryAsync};
use crate::model::{
    common::{GlobalGuard, PostgresPool, QueryOption, QueryOptAsync, RedisPool},
    errors::ServiceError,
    user::{ToUserRef, AuthRequest, UserUpdateRequest},
};

pub fn get_user(jwt: UserJwt, id: Path<u32>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    use crate::model::{user::IdToQuery, cache::IdToUserQuery};
    handle_cache_query(id.into_query_cache(jwt.user_id), &cache)
        .into_future()
        .then(move |res| match res {
            Ok(res) => ft_ok(res),
            Err(_) => id.into_query(jwt.user_id)
                .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
                .into_future()
        })
}

pub fn login_user(req: Json<AuthRequest>, db: Data<PostgresPool>)
                  -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    req.to_login_query()
        .handle_query(&QueryOption::new(Some(&db), None, None))
        .into_future()
}

pub fn update_user(jwt: UserJwt, mut req: Json<UserUpdateRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                   -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    req.attach_id(Some(jwt.user_id))
        .to_query()
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
        .into_future()
}

pub fn register_user(global: Data<GlobalGuard>, req: Json<AuthRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                     -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    req.to_register_query()
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), Some(&global)))
        .into_future()
}

/// async query
use crate::handler::cache::UpdateCache;

pub fn get_user_async(jwt: UserJwt, id: Path<u32>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                      -> impl Future<Item=HttpResponse, Error=Error> {
    use crate::model::{user::IdToQueryAsync, cache::IdToUserQueryAsync};
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

pub fn register_user_async(req: Json<AuthRequest>, global: Data<GlobalGuard>, db: Data<PostgresPool>, cache: Data<RedisPool>)
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

pub fn update_user_async(jwt: UserJwt, req: Json<UserUpdateRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                         -> impl Future<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .attach_id_async(Some(jwt.user_id))
        .into_update_query()
        .into_user(QueryOptAsync::new(Some(db), None))
        .from_err()
        .and_then(move |u| {
            let _ignore = UpdateCache::GotUser(&u).handle_update(&Some(&cache));
            HttpResponse::Ok().json(u.to_ref())
        })
}

pub fn login_user_async(req: Json<AuthRequest>, db: Data<PostgresPool>)
                        -> impl Future<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .into_login_query()
        .into_login(QueryOptAsync::new(Some(db), None))
        .from_err()
        .and_then(|u| HttpResponse::Ok().json(&u))
}