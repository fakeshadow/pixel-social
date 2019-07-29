use actix_web::{HttpResponse, Error, web::{Data, Json, Path}, ResponseError};
use futures::{Future, future::{Either, ok as ft_ok, IntoFuture}};

use crate::handler::{
    auth::UserJwt,
    user::{GetUsers, GetUsersCache},
    post::{GetPosts, GetPostsCache, ModifyPost, AddPostCache},
    cache::UpdateCache,
};
use crate::model::{
    errors::ResError,
    actors::{DB, CACHE},
    common::GlobalVars,
    post::{Post, PostRequest},
};

pub fn add(
    jwt: UserJwt,
    db: Data<DB>,
    cache: Data<CACHE>,
    req: Json<PostRequest>,
    global: Data<GlobalVars>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    jwt.check_privilege()
        .into_future()
        .from_err()
        .and_then(move |_| {
            let req = req.into_inner().attach_user_id(Some(jwt.user_id));
            req.check_new()
                .into_future()
                .from_err()
                .and_then(move |_| db
                    .send(ModifyPost(req, Some(global.get_ref().clone())))
                    .from_err()
                    .and_then(|r| r)
                    .from_err()
                    .and_then(move |p| {
                        let res = HttpResponse::Ok().json(&p);
                        let _ = cache.do_send(AddPostCache(p));
                        res
                    }))
        })
}

pub fn update(
    jwt: UserJwt,
    req: Json<PostRequest>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let mut req = req.into_inner().attach_user_id(Some(jwt.user_id));
    req.check_update()
        .into_future()
        .from_err()
        .and_then(move |_| db
            .send(ModifyPost(req, None))
            .from_err()
            .and_then(|r| r)
            .from_err()
            .and_then(move |p| {
                let res = HttpResponse::Ok().json(&p);
                let _ = cache.do_send(UpdateCache::Post(vec![p]));
                res
            }))
}

pub fn get(
    id: Path<u32>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let id = id.into_inner();
    cache.send(GetPostsCache::Ids(vec![id]))
        .from_err()
        .and_then(move |r| match r {
            Ok((p, i)) => Either::A(attach_users_form_res(i, p, db, cache, false)),
            Err(_) => Either::B(db
                .send(GetPosts(vec![id]))
                .from_err()
                .and_then(|r| r)
                .from_err()
                .and_then(move |(p, i)| attach_users_form_res(i, p, db, cache, true)))
        })
}

fn attach_users_form_res(
    ids: Vec<u32>,
    p: Vec<Post>,
    db: Data<DB>,
    cache: Data<CACHE>,
    update_p: bool,
) -> impl Future<Item=HttpResponse, Error=Error> {
    cache.send(GetUsersCache(ids))
        .from_err()
        .and_then(move |r| match r {
            Ok(u) => {
                let res = HttpResponse::Ok().json(Post::attach_users(&p, &u));
                if update_p {
                    let _ = cache.do_send(UpdateCache::Post(p));
                }
                Either::A(ft_ok(res))
            }
            Err(e) => Either::B(match e {
                ResError::IdsFromCache(ids) => Either::B(db
                    .send(GetUsers(ids))
                    .from_err()
                    .and_then(|r| r)
                    .from_err()
                    .and_then(move |u| {
                        let res = HttpResponse::Ok().json(Post::attach_users(&p, &u));
                        let _ = cache.do_send(UpdateCache::User(u));
                        if update_p {
                            let _ = cache.do_send(UpdateCache::Post(p));
                        }
                        res
                    })),
                _ => Either::A(ft_ok(e.render_response()))
            })
        })
}