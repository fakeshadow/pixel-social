use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, future::{Either, ok as ft_ok}};

use crate::handler::{
    auth::UserJwt,
    post::{GetPost, ModifyPost},
    cache::UpdateCache};
use crate::model::{
    actors::{DB, CACHE},
    common::{GlobalGuard, AttachUser},
    post::PostRequest,
};

pub fn add(
    jwt: UserJwt,
    db: Data<DB>,
    cache: Data<CACHE>,
    req: Json<PostRequest>,
    global: Data<GlobalGuard>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = req.into_inner().attach_user_id_into(Some(jwt.user_id));
    // ToDo: Add trigger before inserting. Make post_id null if the topic doesn't contain target post
    db.send(ModifyPost(req, Some(global.get_ref().clone())))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(move |p| {
            let res = HttpResponse::Ok().json(&p);
            let _ = cache.do_send(UpdateCache::Post(p));
            res
        })
}

pub fn update(
    jwt: UserJwt,
    req: Json<PostRequest>,
    db: Data<DB>,
    cache: Data<CACHE>,
    global: Data<GlobalGuard>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = req.into_inner().attach_user_id_into(Some(jwt.user_id));
    db.send(ModifyPost(req, None))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(move |p| {
            let res = HttpResponse::Ok().json(&p);
            let _ = cache.do_send(UpdateCache::Post(p));
            res
        })
}

pub fn get(
    id: Path<u32>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    db.send(GetPost(id.into_inner()))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(|p| {
            let res = HttpResponse::Ok().json(&p);
            res
        })
}
