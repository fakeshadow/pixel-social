use actix_web::{HttpResponse, Error, web::{Data, Json, Path}, ResponseError};
use futures::{Future, future::{IntoFuture, Either, ok as ft_ok}};

use crate::model::{
    errors::ServiceError,
    actors::{DB, CACHE},
    common::GlobalGuard,
    topic::{TopicRequest, TopicWithPost},
};
use crate::handler::{
    auth::UserJwt,
    user::GetUsers,
    post::GetPosts,
    topic::{AddTopic, UpdateTopic, GetTopics},
    cache::{AddedTopic, UpdateCache, GetTopicCache, GetUsersCache},
};

pub fn get_oldest(
    req: Path<(u32, i64)>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let (tid, page) = req.into_inner();
    get(tid, page, GetTopicCache::Old(tid, page), db, cache)
}

pub fn get_popular(
    req: Path<(u32, i64)>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let (tid, page) = req.into_inner();
    get(tid, page, GetTopicCache::Popular(tid, page), db, cache)
}

fn get(
    tid: u32,
    page: i64,
    msg: GetTopicCache,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    cache.send(msg)
        .from_err()
        .and_then(move |r| match r {
            Err(ids) => {
                if let Some(ids) = ids {
                    return Either::B(db
                        .send(GetPosts(ids))
                        .from_err()
                        .and_then(|r| r)
                        .from_err()
                        .and_then(move |(p, mut u)| {
                            if page == 1 {
                                Either::A(db
                                    .send(GetTopics(vec![tid]))
                                    .from_err()
                                    .and_then(|r| r)
                                    .from_err()
                                    .and_then(move |(t, mut ut)| {
                                        u.append(&mut ut);
                                        cache.send(GetUsersCache(u))
                                            .from_err()
                                            .and_then(move |r| match r {
                                                Ok(u) => {
                                                    let res = HttpResponse::Ok().json(TopicWithPost::new(t.first(), &p, &u));
                                                    let _ = cache.do_send(UpdateCache::Topic(t));
                                                    let _ = cache.do_send(UpdateCache::Post(p));
                                                    println!("topics and posts from db");
                                                    Either::A(ft_ok(res))
                                                }
                                                Err(ids) => Either::B(db
                                                    .send(GetUsers(ids))
                                                    .from_err()
                                                    .and_then(|r| r)
                                                    .from_err()
                                                    .and_then(move |u| {
                                                        let res = HttpResponse::Ok().json(TopicWithPost::new(None, &p, &u));
                                                        let _ = cache.do_send(UpdateCache::Topic(t));
                                                        let _ = cache.do_send(UpdateCache::Post(p));
                                                        let _ = cache.do_send(UpdateCache::User(u));
                                                        println!("topics,posts and users from db");
                                                        res
                                                    })
                                                )
                                            })
                                    })
                                )
                            } else {
                                Either::B(cache
                                    .send(GetUsersCache(u))
                                    .from_err()
                                    .and_then(move |r| match r {
                                        Ok(u) => {
                                            let res = HttpResponse::Ok().json(TopicWithPost::new(None, &p, &u));
                                            let _ = cache.do_send(UpdateCache::Post(p));
                                            println!("posts from db");

                                            Either::A(ft_ok(res))
                                        }
                                        Err(ids) => Either::B(db
                                            .send(GetUsers(ids))
                                            .from_err()
                                            .and_then(|r| r)
                                            .from_err()
                                            .and_then(move |u| {
                                                let res = HttpResponse::Ok().json(TopicWithPost::new(None, &p, &u));
                                                let _ = cache.do_send(UpdateCache::Post(p));
                                                let _ = cache.do_send(UpdateCache::User(u));
                                                println!("posts and users from db");
                                                res
                                            })
                                        )
                                    })
                                )
                            }
                        })
                    );
                }

                Either::A(ft_ok(ServiceError::NoContent.render_response()))
            }
            Ok((t, p, u)) => {
                let topic = if page == 1 { Some(&t) } else { None };
                Either::A(ft_ok(HttpResponse::Ok().json(TopicWithPost::new(topic, &p, &u))))
            }
        })
}

pub fn add(
    jwt: UserJwt,
    db: Data<DB>,
    cache: Data<CACHE>,
    req: Json<TopicRequest>,
    global: Data<GlobalGuard>,
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
                    .send(AddTopic(req, global.get_ref().clone()))
                    .from_err()
                    .and_then(|r| r)
                    .from_err()
                    .and_then(move |t| {
                        let res = HttpResponse::Ok().json(&t);
                        let _ = cache.do_send(AddedTopic(t));
                        res
                    }))
        })
}

pub fn update(
    jwt: UserJwt,
    db: Data<DB>,
    cache: Data<CACHE>,
    req: Json<TopicRequest>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let mut req = req.into_inner().attach_user_id(Some(jwt.user_id));
    req.check_update()
        .into_future()
        .from_err()
        .and_then(move |_| db
            .send(UpdateTopic(req))
            .from_err()
            .and_then(|r| r)
            .from_err()
            .and_then(move |t| {
                let res = HttpResponse::Ok().json(&t);
                let _ = cache.do_send(UpdateCache::Topic(vec![t]));
                res
            }))
}