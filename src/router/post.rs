use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, future::{Either, ok as ft_ok}};

use crate::handler::{auth::UserJwt, cache::{UpdateCache, get_unique_users_cache}};
use crate::model::{
    common::{GlobalGuard, PostgresPool, RedisPool, Response, AttachUser},
    post::PostRequest,
};
use crate::handler::user::get_unique_users;

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
        .into_add_post(&db, Some(global))
        .from_err()
        .and_then(move |(c, t, p, p_new)| {
            let _ignore = UpdateCache::AddedPost(&t, &c, &p_new, &p).handle_update(&Some(&cache));
            Response::AddedPost.to_res()
        })
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
        .into_post(&db)
        .from_err()
        .and_then(move |p| {
            let _ignore = UpdateCache::GotPost(&p).handle_update(&Some(&cache));
            Response::AddedPost.to_res()
        })
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
                get_unique_users_cache(&p, None, &cache)
                    .from_err()
                    .and_then(move |u|
                        HttpResponse::Ok().json(&p.first().unwrap().attach_user(&u)))),
            Err(_) => Either::B(
                id.to_query()
                    .into_post(&db)
                    .from_err()
                    .and_then(move |p| {
                        let p = vec![p];
                        get_unique_users(&p, None, &db)
                            .from_err()
                            .and_then(move |u|
                                HttpResponse::Ok().json(&p.first().unwrap().attach_user(&u)))
                    }))
        })
}
