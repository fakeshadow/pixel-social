use actix_web::{HttpResponse, Error, web::{Data, Path}, ResponseError};
use futures::{Future, future::{Either, ok as ft_ok}};

use crate::handler::{
    db::DatabaseService,
    cache::CacheService
};
use crate::model::{
    errors::ResError,
    topic::Topic,
};

pub fn get_all(
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    cache.get_categories_all()
        .then(move |r| match r {
            Ok(c) => Either::A(ft_ok(HttpResponse::Ok().json(&c))),
            Err(_) => Either::B(db
                .get_categories_all()
                .from_err()
                .and_then(move |c| {
                    let res = HttpResponse::Ok().json(&c);
                    cache.update_categories(c);
                    res
                })
            )
        })
}

pub fn get_latest(
    req: Path<(u32, i64)>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let (id, page) = req.into_inner();
    cache.get_topics_late(id, page)
        .then(move |r| get(db, cache, r))
}

pub fn get_popular(
    req: Path<(u32, i64)>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let (id, page) = req.into_inner();

    cache.get_topics_pop(id, page)
        .then(move |r| get(db, cache, r))
}

pub fn get_popular_all(
    req: Path<(i64)>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let page = req.into_inner();

    cache.get_topics_pop_all(page)
        .then(move |r| get(db, cache, r))
}

fn get(
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    result: Result<(Vec<Topic>, Vec<u32>), ResError>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    match result {
        Ok((t, ids)) => Either::A(
            attach_users_form_res(ids, t, db, cache, false)
        ),
        Err(e) => Either::B(match e {
            ResError::IdsFromCache(ids) => Either::B(db
                .get_by_id_with_uid(&db.topics_by_id, ids)
                .from_err()
                .and_then(move |(t, ids)|
                    attach_users_form_res(ids, t, db, cache, true)
                )
            ),
            _ => Either::A(ft_ok(e.render_response()))
        })
    }
}

fn attach_users_form_res(
    ids: Vec<u32>,
    t: Vec<Topic>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    update_t: bool,
) -> impl Future<Item=HttpResponse, Error=Error> {
    cache.get_users_from_ids(ids)
        .then(move |r| match r {
            Ok(u) => {
                let res = HttpResponse::Ok().json(Topic::attach_users(&t, &u));
                if update_t {
                    cache.update_topics(t);
                }
                Either::A(ft_ok(res))
            }
            Err(e) => Either::B(match e {
                ResError::IdsFromCache(ids) => Either::B(db
                    .get_by_id(&db.users_by_id, &ids)
                    .from_err()
                    .and_then(move |u| {
                        let res = HttpResponse::Ok().json(Topic::attach_users(&t, &u));
                        cache.update_users(u);
                        if update_t {
                            cache.update_topics(t);
                        }
                        res
                    })),
                _ => Either::A(ft_ok(e.render_response()))
            })
        })
}