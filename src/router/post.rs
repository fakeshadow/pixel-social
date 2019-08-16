use actix_web::{
    web::{Data, Json, Path},
    Error, HttpResponse, ResponseError,
};
use futures::{
    future::{ok as ft_ok, Either, IntoFuture},
    Future,
};

use crate::handler::auth::UserJwt;
use crate::handler::cache::CacheService;
use crate::handler::db::DatabaseService;
use crate::model::{
    common::GlobalVars,
    errors::ResError,
    post::{Post, PostRequest},
};

pub fn add(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Json<PostRequest>,
    global: Data<GlobalVars>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    jwt.check_privilege()
        .into_future()
        .from_err()
        .and_then(move |_| {
            req.into_inner()
                .attach_user_id(Some(jwt.user_id))
                .check_new()
                .into_future()
                .from_err()
                .and_then(move |req| {
                    db.add_post(req, global.get_ref())
                        .from_err()
                        .and_then(move |p| {
                            let res = HttpResponse::Ok().json(&p);
                            cache.add_post(p);
                            res
                        })
                })
        })
}

pub fn update(
    jwt: UserJwt,
    req: Json<PostRequest>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    req.into_inner()
        .attach_user_id(Some(jwt.user_id))
        .check_update()
        .into_future()
        .from_err()
        .and_then(move |req| {
            db.update_post(req).from_err().and_then(move |p| {
                let res = HttpResponse::Ok().json(&p);
                cache.update_posts(vec![p]);
                res
            })
        })
}

pub fn get(
    id: Path<u32>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let id = id.into_inner();
    cache.get_posts_from_ids(vec![id]).then(move |r| match r {
        Ok((p, i)) => Either::A(attach_users_form_res(i, p, db, cache, false)),
        Err(_) => Either::B(
            db.get_by_id_with_uid(&db.posts_by_id, vec![id])
                .from_err()
                .and_then(move |(p, i)| attach_users_form_res(i, p, db, cache, true)),
        ),
    })
}

fn attach_users_form_res(
    ids: Vec<u32>,
    p: Vec<Post>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    update_p: bool,
) -> impl Future<Item = HttpResponse, Error = Error> {
    cache.get_users_from_ids(ids).then(move |r| match r {
        Ok(u) => {
            let res = HttpResponse::Ok().json(Post::attach_users(&p, &u));
            if update_p {
                cache.update_posts(p);
            }
            Either::A(ft_ok(res))
        }
        Err(e) => Either::B(match e {
            ResError::IdsFromCache(ids) => Either::B(
                db.get_by_id(&db.users_by_id, &ids)
                    .from_err()
                    .and_then(move |u| {
                        let res = HttpResponse::Ok().json(Post::attach_users(&p, &u));
                        cache.update_users(u);
                        if update_p {
                            cache.update_posts(p);
                        }
                        res
                    }),
            ),
            _ => Either::A(ft_ok(e.render_response())),
        }),
    })
}
