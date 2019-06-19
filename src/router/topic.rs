use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, future::Either};

use crate::model::{
    actors::{DB, CACHE},
    common::{GlobalGuard, AttachUser},
    topic::{TopicRequest, TopicWithPost},
};

use crate::handler::{
    auth::UserJwt,
    user::GetUsers,
    topic::{AddTopic, UpdateTopic,GetTopicWithPost},
    cache::{AddedTopic, UpdateCache},
};

pub fn get(
    req: Path<(u32, u32, i64)>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let (cid, tid, page) = req.into_inner();

    db.send(GetTopicWithPost(cid, tid, page))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(move |((t, mut tids), (p, mut ids))| {
            ids.push(tids.pop().unwrap_or(0));
            ids.sort();
            ids.dedup();
            db.send(GetUsers(ids))
                .from_err()
                .and_then(|r| r)
                .from_err()
                .and_then(move |u| {
                    // include topic when querying first page.
                    let topic = if page == 1 { t.first() } else { None };
                    HttpResponse::Ok().json(TopicWithPost::new(topic, &p, &u))
                })
        })
}

pub fn add(
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

pub fn update(
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