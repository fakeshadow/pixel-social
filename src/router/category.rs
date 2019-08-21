use actix_web::{
    web::{Data, Query},
    Error, HttpResponse, ResponseError,
};
use futures::{
    future::{ok as ft_ok, Either},
    Future,
};

use crate::{
    handler::{cache::CacheService, db::DatabaseService},
    model::{
        category::{CategoryQuery, QueryType},
        errors::ResError,
        topic::Topic,
    },
};

pub fn query_handler(
    req: Query<CategoryQuery>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    match req.query_type {
        QueryType::PopularAll => Either::A(Either::A(
            cache
                .get_topics_pop_all(req.page.unwrap_or(1))
                .then(move |r| if_query_db(db, cache, r)),
        )),
        QueryType::Popular => Either::A(Either::B(
            cache
                .get_topics_pop(req.category_id.unwrap_or(1), req.page.unwrap_or(1))
                .then(move |r| if_query_db(db, cache, r)),
        )),
        QueryType::Latest => Either::B(Either::A(
            cache
                .get_topics_late(req.category_id.unwrap_or(1), req.page.unwrap_or(1))
                .then(move |r| if_query_db(db, cache, r)),
        )),
        QueryType::All => Either::B(Either::B(cache.get_categories_all().then(
            move |r| match r {
                Ok(c) => Either::A(ft_ok(HttpResponse::Ok().json(&c))),
                Err(_) => Either::B(db.get_categories_all().from_err().and_then(move |c| {
                    let res = HttpResponse::Ok().json(&c);
                    cache.update_categories(&c);
                    res
                })),
            },
        ))),
    }
}

fn if_query_db(
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    result: Result<(Vec<Topic>, Vec<u32>), ResError>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    match result {
        Ok((t, ids)) => Either::A(attach_users_form_res(ids, t, db, cache, false)),
        Err(e) => Either::B(match e {
            ResError::IdsFromCache(ids) => Either::B(
                db.get_by_id_with_uid(&db.topics_by_id, ids)
                    .from_err()
                    .and_then(move |(t, ids)| attach_users_form_res(ids, t, db, cache, true)),
            ),
            _ => Either::A(ft_ok(e.render_response())),
        }),
    }
}

fn attach_users_form_res(
    ids: Vec<u32>,
    t: Vec<Topic>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    update_t: bool,
) -> impl Future<Item = HttpResponse, Error = Error> {
    cache.get_users_from_ids(ids).then(move |r| match r {
        Ok(u) => {
            if update_t {
                cache.update_topics(&t);
            }
            Either::A(ft_ok(HttpResponse::Ok().json(Topic::attach_users(&t, &u))))
        }
        Err(e) => Either::B(match e {
            ResError::IdsFromCache(ids) => Either::B(
                db.get_by_id(&db.users_by_id, &ids)
                    .from_err()
                    .and_then(move |u| {
                        cache.update_users(&u);
                        if update_t {
                            cache.update_topics(&t);
                        }
                        HttpResponse::Ok().json(Topic::attach_users(&t, &u))
                    }),
            ),
            _ => Either::A(ft_ok(e.render_response())),
        }),
    })
}
