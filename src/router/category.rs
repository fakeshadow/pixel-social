use actix_web::{HttpResponse, Error, web::{Data, Path}, ResponseError};
use futures::{Future, future::{Either, ok as ft_ok}};

use crate::handler::{
    cache::{GetCategoriesCache, GetTopicsCache, UpdateCache, GetUsersCache},
    topic::GetTopics,
    category::GetCategories,
    user::GetUsers,
};
use crate::model::{
    errors::ServiceError,
    actors::{DB, CACHE},
    topic::Topic,
};

pub fn get_all(
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    cache.send(GetCategoriesCache)
        .from_err()
        .and_then(move |r| match r {
            Ok(c) => Either::A(ft_ok(HttpResponse::Ok().json(&c))),
            Err(_) => Either::B(db.send(GetCategories)
                .from_err()
                .and_then(|r| r)
                .from_err()
                .and_then(move |c| {
                    let res = HttpResponse::Ok().json(&c);
                    let _ = cache.do_send(UpdateCache::Category(c));
                    res
                }))
        })
}

pub fn get_latest(
    req: Path<(u32, i64)>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let (id, page) = req.into_inner();

    get(db, cache, GetTopicsCache::Latest(id, page))
}

pub fn get_popular(
    req: Path<(u32, i64)>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let (id, page) = req.into_inner();

    get(db, cache, GetTopicsCache::Popular(id, page))
}

pub fn get_popular_all(
    req: Path<(i64)>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let page = req.into_inner();

    get(db, cache, GetTopicsCache::PopularAll(page))
}

fn get(
    db: Data<DB>,
    cache: Data<CACHE>,
    msg: GetTopicsCache,
) -> impl Future<Item=HttpResponse, Error=Error> {
    cache.send(msg)
        .from_err()
        .and_then(move |r| match r {
            Ok((t, ids)) => Either::A(attach_users_form_res(ids, t, db, cache, false)),
            Err(e) => Either::B(match e {
                ServiceError::IdsFromCache(ids) => Either::B(topics_from_db(ids, db, cache)),
                _ => Either::A(ft_ok(e.render_response()))
            })
        })
}

fn topics_from_db(
    ids: Vec<u32>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    db.send(GetTopics(ids))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(move |(t, ids)| attach_users_form_res(ids, t, db, cache, true))
}

fn attach_users_form_res(
    ids: Vec<u32>,
    t: Vec<Topic>,
    db: Data<DB>,
    cache: Data<CACHE>,
    update_t: bool,
) -> impl Future<Item=HttpResponse, Error=Error> {
    cache.send(GetUsersCache(ids))
        .from_err()
        .and_then(move |r| match r {
            Ok(u) => {
                let res = HttpResponse::Ok().json(Topic::attach_users(&t, &u));
                if update_t {
                    let _ = cache.do_send(UpdateCache::Topic(t));
                }
                Either::A(ft_ok(res))
            }
            Err(ids) => Either::B(db
                .send(GetUsers(ids))
                .from_err()
                .and_then(|r| r)
                .from_err()
                .and_then(move |u| {
                    let res = HttpResponse::Ok().json(Topic::attach_users(&t, &u));
                    let _ = cache.do_send(UpdateCache::User(u));
                    if update_t {
                        let _ = cache.do_send(UpdateCache::Topic(t));
                    }
                    res
                })
            )
        })
}