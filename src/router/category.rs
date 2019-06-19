use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, future::{Either, ok as ft_ok}};

use crate::handler::{
    auth::UserJwt,
    cache::{GetCategoriesCache, GetTopicsCache, UpdateCache},
    topic::GetTopics,
    category::GetCategories,
    user::GetUsers,
};
use crate::model::{
    actors::{DB, CACHE},
    topic::TopicWithUser,
    common::AttachUser,
};

pub fn get_popular(
    page: Path<(i64)>
) -> impl Future<Item=HttpResponse, Error=Error> {
    // ToDo: Add get popular cache query
    ft_ok(HttpResponse::Ok().finish())
}

pub fn get_all_categories(
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

pub fn get_category(
    req: Path<(u32, i64)>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let (id, page) = req.into_inner();
    cache.send(GetTopicsCache(vec![id], page))
        .from_err()
        .and_then(move |r| match r {
            Ok((t, u)) => Either::A(ft_ok(HttpResponse::Ok().json(&t.iter()
                .map(|t| t.attach_user(&u))
                .collect::<Vec<TopicWithUser>>()))),
            Err(_) => Either::B(db.send(GetTopics::Latest(id, page))
                .from_err()
                .and_then(|r| r)
                .from_err()
                // return user ids with topics for users query
                .and_then(move |(t, ids)| db
                    .send(GetUsers(ids))
                    .from_err()
                    .and_then(|r| r)
                    .from_err()
                    .and_then(move |u| {
                        let res = HttpResponse::Ok().json(&t.iter()
                            .map(|t| t.attach_user(&u))
                            .collect::<Vec<TopicWithUser>>());
                        let _ = cache.do_send(UpdateCache::Topic(t));
                        let _ = cache.do_send(UpdateCache::User(u));
                        println!("from db");
                        res
                    })
                ))
        })
}