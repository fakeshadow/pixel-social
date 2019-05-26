use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, future::{Either, ok as ft_ok}};

use crate::handler::{
    auth::UserJwt,
    user::get_unique_users,
    cache::{UpdateCacheAsync, get_unique_users_cache}};
use crate::model::{
    common::{GlobalGuard, PostgresPool, RedisPool, AttachUser},
    post::PostRequest,
};

// ToDo: Return post on response and use it to update the frontend.
pub fn add_post(
    jwt: UserJwt,
    req: Json<PostRequest>,
    db: Data<PostgresPool>,
    cache: Data<RedisPool>,
    global: Data<GlobalGuard>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .attach_user_id_into(Some(jwt.user_id))
        .into_add_query()
        .into_add_post(db.get_ref().clone(), Some(global.get_ref().clone()))
        .from_err()
        .and_then(move |(c, t, p, p_new)|
            UpdateCacheAsync::AddedPost(c, t, p, p_new)
                .handler(&cache)
                .then(|_| HttpResponse::Ok().finish()))
}

pub fn update_post(
    jwt: UserJwt,
    req: Json<PostRequest>,
    db: Data<PostgresPool>,
    cache: Data<RedisPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .attach_user_id_into(Some(jwt.user_id))
        .into_update_query()
        .into_post(db.get_ref().clone())
        .from_err()
        .and_then(move |p|
            UpdateCacheAsync::GotPost(p)
                .handler(&cache)
                .then(|_| HttpResponse::Ok().finish()))
}

pub fn get_post(
    id: Path<u32>,
    db: Data<PostgresPool>,
    cache: Data<RedisPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    use crate::model::{cache::IdToPostQuery, post::IdToQuery};
    id.to_query_cache()
        .into_post(&cache)
        .then(move |r| match r {
            Ok(p) => Either::A(
                get_unique_users_cache(&p, None, cache.get_ref().clone())
                    .from_err()
                    .and_then(move |u|
                        HttpResponse::Ok().json(&p.first().unwrap().attach_user(&u)))),
            Err(_) => Either::B(
                id.to_query()
                    .into_post(db.get_ref().clone())
                    .from_err()
                    .and_then(move |p| {
                        let mut p = vec![p];
                        get_unique_users(&p, None, &db)
                            .from_err()
                            .and_then(move |u| {
                                let res = HttpResponse::Ok().json(&p.first().unwrap().attach_user(&u));
                                UpdateCacheAsync::GotPost(p.pop().unwrap()).handler(&cache).then(|_| res)
                            })
                    }))
        })
}
