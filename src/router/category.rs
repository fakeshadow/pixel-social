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
    topic::TopicWithUser,
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

    cache.send(GetTopicsCache::Latest(id, page))
        .from_err()
        .and_then(move |r| match r {
            Ok((t, u)) => Either::A(ft_ok(HttpResponse::Ok().json(TopicWithUser::new(&t, &u)))),
            Err(ids) => {
                if let Some(ids) = ids {
                    return Either::B(from_db(ids, db, cache));
                }
                Either::A(ft_ok(ServiceError::NoContent.render_response()))
            }
        })
}

pub fn get_popular(
    req: Path<(u32, i64)>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let (id, page) = req.into_inner();

    cache.send(GetTopicsCache::Popular(id, page))
        .from_err()
        .and_then(move |r| match r {
            Ok((t, u)) => Either::A(ft_ok(HttpResponse::Ok().json(TopicWithUser::new(&t, &u)))),
            Err(ids) => {
                if let Some(ids) = ids {
                    return Either::B(from_db(ids, db, cache));
                }
                Either::A(ft_ok(ServiceError::NoContent.render_response()))
            }
        })
}

pub fn get_popular_all(
    req: Path<(i64)>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let page = req.into_inner();

    cache.send(GetTopicsCache::PopularAll(page))
        .from_err()
        .and_then(move |r| match r {
            Ok((t, u)) => Either::A(ft_ok(HttpResponse::Ok().json(TopicWithUser::new(&t, &u)))),
            Err(ids) => {
                if let Some(ids) = ids {
                    return Either::B(from_db(ids, db, cache));
                }
                Either::A(ft_ok(ServiceError::NoContent.render_response()))
            }
        })
}

fn from_db(
    ids: Vec<u32>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    db.send(GetTopics(ids))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(move |(t, ids)| cache
            .send(GetUsersCache(ids))
            .from_err()
            .and_then(move |r| match r {
                Ok(u) => {
                    let res = HttpResponse::Ok().json(TopicWithUser::new(&t, &u));
                    let _ = cache.do_send(UpdateCache::Topic(t));
                    println!("topics from db");
                    Either::A(ft_ok(res))
                }
                Err(ids) => Either::B(db
                    .send(GetUsers(ids))
                    .from_err()
                    .and_then(|r| r)
                    .from_err()
                    .and_then(move |u| {
                        let res = HttpResponse::Ok().json(TopicWithUser::new(&t, &u));
                        let _ = cache.do_send(UpdateCache::Topic(t));
                        let _ = cache.do_send(UpdateCache::User(u));
                        println!("topics and users from db");
                        res
                    })
                )
            })
        )
}