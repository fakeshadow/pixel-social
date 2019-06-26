use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, future::{Either, ok as ft_ok, IntoFuture}};

use crate::handler::{
    auth::UserJwt,
    user::GetUsers,
    post::{GetPosts, ModifyPost},
    cache::{UpdateCache, GetPostsCache, AddedPost},
};
use crate::model::{
    actors::{DB, CACHE},
    common::{GlobalGuard, AttachUser},
    post::{PostRequest, PostWithUser},
};

pub fn add(
    jwt: UserJwt,
    db: Data<DB>,
    cache: Data<CACHE>,
    req: Json<PostRequest>,
    global: Data<GlobalGuard>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = req.into_inner().attach_user_id(Some(jwt.user_id));
    // ToDo: Add trigger before inserting. Make post_id null if the topic doesn't contain target post
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
                let _ = cache.do_send(AddedPost(p));
                res
            }))
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
    cache.send(GetPostsCache(vec![id]))
        .from_err()
        .and_then(move |r| match r {
            Ok((p, u)) => Either::A(ft_ok(HttpResponse::Ok().json(&p
                .iter()
                .map(|p| p.attach_user(&u))
                .collect::<Vec<PostWithUser>>()))),
            Err(_) => Either::B(db
                .send(GetPosts(vec![id]))
                .from_err()
                .and_then(|r| r)
                .from_err()
                .and_then(move |(p, ids)| {
                    db.send(GetUsers(ids))
                        .from_err()
                        .and_then(|r| r)
                        .from_err()
                        .and_then(move |u| {
                            let res = HttpResponse::Ok().json(&p
                                .iter()
                                .map(|p| p.attach_user(&u))
                                .collect::<Vec<PostWithUser>>());
                            let _ = cache.do_send(UpdateCache::Post(p));
                            res
                        })
                }))
        })
}
