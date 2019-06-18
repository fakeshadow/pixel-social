use futures::{Future, future::{Either, IntoFuture, ok as ft_ok, err as ft_err}};

use actix_web::{Error, HttpResponse, ResponseError, web::{Data, Json, Path}};

use crate::model::{
    actors::{PostgresConnection, DB, CACHE},
    user::{AuthRequest, UpdateRequest},
    common::{AttachUser, GlobalGuard, PostgresPool, RedisPool, Validator},
    topic::{TopicRequest, TopicQuery, TopicWithUser, TopicWithPost},
};
use crate::handler::{
    auth::UserJwt,
    user::GetUsers,
    db::{GetTopics, GetCategories, GetTopic, GetPosts, AddTopic, UpdateTopic},
    cache::{UpdateCacheAsync, GetCategoriesCache, GetTopicsCache, UpdateCache, AddedTopic},
};

pub fn hello_world() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().json("hello world"))
}

pub fn test_global_var(
    global: Data<GlobalGuard>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = TopicRequest {
        id: None,
        user_id: Some(1),
        category_id: Some(1),
        thumbnail: Some("test thumbnail".to_string()),
        title: Some("test title".to_string()),
        body: Some("test body".to_string()),
        is_locked: None,
    };
    db.send(AddTopic(req, global.get_ref().clone()))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(move |t| {
            let cid = t.category_id;
            let res = HttpResponse::Ok().json(&t);
            let _ = cache.do_send(AddedTopic(t, cid));
            res
        })
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

pub fn get_topic(
    req: Path<(u32, i64)>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let (id, page) = req.into_inner();
    db.send(GetTopic(id))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(move |t| db
            .send(GetPosts(id, page))
            .from_err()
            .and_then(|r| r)
            .from_err()
            // return user ids and posts for users query
            .and_then(move |(p, mut ids)| {
                // push topic's user_id and sort user ids
                if let Some(t) = t.first().as_ref() {
                    ids.push(t.user_id);
                    ids.sort();
                    ids.dedup();
                };
                db.send(GetUsers(ids))
                    .from_err()
                    .and_then(|r| r)
                    .from_err()
                    .and_then(move |u| {
                        // include topic when querying first page.
                        let topic = if page == 1 { t.first() } else { None };
                        HttpResponse::Ok().json(TopicWithPost::new(topic, &p, &u))
                    })
            }))
}

pub fn add_topic(
    jwt: UserJwt,
    db: Data<DB>,
    cache: Data<CACHE>,
    req: Json<TopicRequest>,
    global: Data<GlobalGuard>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = req.into_inner().attach_user_id_into(Some(jwt.user_id));
    db.send(AddTopic(req, global.get_ref().clone()))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(move |t| {
            let cid = t.category_id;
            let res = HttpResponse::Ok().json(&t);
            let _ = cache.do_send(AddedTopic(t, cid));
            res
        })
}

pub fn update_topic(
    jwt: UserJwt,
    db: Data<DB>,
    cache: Data<CACHE>,
    req: Json<TopicRequest>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = req.into_inner().attach_user_id_into(Some(jwt.user_id));
    db.send(UpdateTopic(req))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(move |t| {
            let res = HttpResponse::Ok().json(&t);
            let _ = cache.do_send(UpdateCache::Topic(t));
            res
        })
}