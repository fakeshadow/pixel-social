use actix_web::{HttpResponse, Error, web::{Data, Json, Path}, ResponseError};
use futures::{Future, future::{IntoFuture, Either, ok as ft_ok}};

use crate::model::{
    errors::ServiceError,
    actors::{DB, CACHE},
    common::GlobalGuard,
    post::Post,
    topic::{Topic, TopicRequest},
};
use crate::handler::{
    auth::UserJwt,
    user::GetUsers,
    post::GetPosts,
    topic::{AddTopic, UpdateTopic, GetTopics},
    cache::{AddedTopic, UpdateCache, GetTopicCache, GetTopicsCache, GetUsersCache},
};

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
            Ok((p, ids)) => Either::A({
                if page == 1 {
                    Either::A(get_topic_attach_user_form_res(db, cache, tid, ids, p, false))
                } else {
                    Either::B(attach_user_form_res(db, cache, ids, vec![], p, false, false))
                }
            }),
            Err(e) => Either::B(match e {
                ServiceError::IdsFromCache(ids) => Either::B(db
                    .send(GetPosts(ids))
                    .from_err()
                    .and_then(|r| r)
                    .from_err()
                    .and_then(move |(p, ids)| {
                        if page == 1 {
                            Either::A(get_topic_attach_user_form_res(db, cache, tid, ids, p, true))
                        } else {
                            Either::B(attach_user_form_res(db, cache, ids, vec![], p, false, true))
                        }
                    })),
                _ => Either::A(ft_ok(e.render_response()))
            })
        })
}

fn get_topic_attach_user_form_res(
    db: Data<DB>,
    cache: Data<CACHE>,
    tid: u32,
    mut ids: Vec<u32>,
    p: Vec<Post>,
    update_p: bool,
) -> impl Future<Item=HttpResponse, Error=Error> {
    cache.send(GetTopicsCache::Ids(vec![tid]))
        .from_err()
        .and_then(move |r| match r {
            Ok((t, mut id)) => {
                ids.append(&mut id);
                Either::A(attach_user_form_res(db, cache, ids, t, p, false, update_p))
            }
            Err(e) => Either::B(match e {
                ServiceError::IdsFromCache(tids) => Either::A(db
                    .send(GetTopics(tids))
                    .from_err()
                    .and_then(|r| r)
                    .from_err()
                    .and_then(move |(t, mut id)| {
                        ids.append(&mut id);
                        attach_user_form_res(db, cache, ids, t, p, true, update_p)
                    })
                ),
                _ => Either::B(ft_ok(e.render_response()))
            })
        })
}


fn attach_user_form_res(
    db: Data<DB>,
    cache: Data<CACHE>,
    ids: Vec<u32>,
    t: Vec<Topic>,
    p: Vec<Post>,
    update_t: bool,
    update_p: bool,
) -> impl Future<Item=HttpResponse, Error=Error> {
    cache.send(GetUsersCache(ids))
        .from_err()
        .and_then(move |r| match r {
            Ok(u) => {
                let res = HttpResponse::Ok().json(Topic::attach_users_with_post(t.first(), &p, &u));
                if update_t {
                    let _ = cache.do_send(UpdateCache::Topic(t));
                }
                if update_p {
                    let _ = cache.do_send(UpdateCache::Post(p));
                }
                Either::A(ft_ok(res))
            }
            Err(ids) => Either::B(db
                .send(GetUsers(ids))
                .from_err()
                .and_then(|r| r)
                .from_err()
                .and_then(move |u| {
                    let res = HttpResponse::Ok().json(Topic::attach_users_with_post(t.first(), &p, &u));
                    if update_t {
                        let _ = cache.do_send(UpdateCache::Topic(t));
                    }
                    if update_p {
                        let _ = cache.do_send(UpdateCache::Post(p));
                    }
                    let _ = cache.do_send(UpdateCache::User(u));
                    res
                })
            )
        })
}