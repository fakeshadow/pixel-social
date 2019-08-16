use actix_web::{
    web::{Data, Json, Path},
    Error, HttpResponse, ResponseError,
};
use futures::{
    future::{ok as ft_ok, Either, IntoFuture},
    Future,
};

use crate::handler::{auth::UserJwt, cache::CacheService, db::DatabaseService};
use crate::model::{
    common::GlobalVars,
    errors::ResError,
    post::Post,
    topic::{Topic, TopicRequest},
};

pub fn add(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Json<TopicRequest>,
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
                .and_then(move |req| db.add_topic(&req, global.get_ref()))
                .from_err()
                .and_then(move |t| {
                    let res = HttpResponse::Ok().json(&t);
                    cache.add_topic(t);
                    res
                })
        })
}

pub fn update(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Json<TopicRequest>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    req.into_inner()
        .attach_user_id(Some(jwt.user_id))
        .check_update()
        .into_future()
        .from_err()
        .and_then(move |r| {
            db.update_topic(&r).from_err().and_then(move |t| {
                let res = HttpResponse::Ok().json(&t);
                cache.update_topics(vec![t]);
                res
            })
        })
}

pub fn get_oldest(
    req: Path<(u32, usize)>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let (tid, page) = req.into_inner();

    cache
        .get_posts_old(tid, page)
        .then(move |r| get(tid, page, db, cache, r))
}

pub fn get_popular(
    req: Path<(u32, usize)>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let (tid, page) = req.into_inner();
    cache
        .get_posts_pop(tid, page)
        .then(move |r| get(tid, page, db, cache, r))
}

fn get(
    tid: u32,
    page: usize,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    result: Result<(Vec<Post>, Vec<u32>), ResError>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    match result {
        Ok((p, ids)) => Either::A({
            if page == 1 {
                Either::A(get_topic_attach_user_form_res(
                    db, cache, tid, ids, p, false,
                ))
            } else {
                Either::B(attach_user_form_res(
                    db,
                    cache,
                    ids,
                    vec![],
                    p,
                    false,
                    false,
                ))
            }
        }),
        Err(e) => Either::B(match e {
            ResError::IdsFromCache(ids) => Either::B(
                db.get_by_id_with_uid(&db.posts_by_id, ids)
                    .from_err()
                    .and_then(move |(p, ids)| {
                        if page == 1 {
                            Either::A(get_topic_attach_user_form_res(db, cache, tid, ids, p, true))
                        } else {
                            Either::B(attach_user_form_res(db, cache, ids, vec![], p, false, true))
                        }
                    }),
            ),
            _ => Either::A(ft_ok(e.render_response())),
        }),
    }
}

fn get_topic_attach_user_form_res(
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    tid: u32,
    mut ids: Vec<u32>,
    p: Vec<Post>,
    update_p: bool,
) -> impl Future<Item = HttpResponse, Error = Error> {
    cache.get_topics_from_ids(vec![tid]).then(move |r| match r {
        Ok((t, mut id)) => {
            ids.append(&mut id);
            Either::A(attach_user_form_res(db, cache, ids, t, p, false, update_p))
        }
        Err(e) => Either::B(match e {
            ResError::IdsFromCache(tids) => Either::A(
                db.get_by_id_with_uid(&db.topics_by_id, tids)
                    .from_err()
                    .and_then(move |(t, mut id)| {
                        ids.append(&mut id);
                        attach_user_form_res(db, cache, ids, t, p, true, update_p)
                    }),
            ),
            _ => Either::B(ft_ok(e.render_response())),
        }),
    })
}

fn attach_user_form_res(
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    ids: Vec<u32>,
    t: Vec<Topic>,
    p: Vec<Post>,
    update_t: bool,
    update_p: bool,
) -> impl Future<Item = HttpResponse, Error = Error> {
    cache.get_users_from_ids(ids).then(move |r| match r {
        Ok(u) => {
            let res = HttpResponse::Ok().json(Topic::attach_users_with_post(t.first(), &p, &u));
            if update_t {
                cache.update_topics(t);
            }
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
                        let res = HttpResponse::Ok().json(Topic::attach_users_with_post(
                            t.first(),
                            &p,
                            &u,
                        ));
                        cache.update_users(u);
                        if update_t {
                            cache.update_topics(t);
                        }
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
