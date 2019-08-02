use actix::prelude::*;
use actix_web::{Error, HttpResponse, web::Data};

use crate::model::{
    actors::{DB, CACHE},
    common::GlobalVars,
    topic::TopicRequest,
    post::PostRequest,
};
use crate::handler::{
    topic::{AddTopic, AddTopicCache},
    post::{AddPostCache, ModifyPost},
};

pub fn hello_world() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().json("hello world"))
}

pub fn add_topic(
    global: Data<GlobalVars>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = TopicRequest {
        id: None,
        user_id: Some(1),
        category_id: 1,
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
            let res = HttpResponse::Ok().json(&t);
            let _ = cache.do_send(AddTopicCache(t));
            res
        })
}

pub fn add_post(
    global: Data<GlobalVars>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = PostRequest {
        id: None,
        user_id: Some(1),
        topic_id: Some(1),
        category_id: 1,
        post_id: Some(1),
        post_content: Some("t4265335423646e".to_owned()),
        is_locked: None,
    };
    db.send(ModifyPost(req, Some(global.get_ref().clone())))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(move |p| {
            let res = HttpResponse::Ok().json(&p);
            let _ = cache.do_send(AddPostCache(p));
            res
        })
}

use crate::model::topic::Topic;
use crate::model::errors::ResError;
use std::convert::TryFrom;
use crate::handler::topic::GetTopics;

type Pool = bb8::Pool<bb8_postgres::PostgresConnectionManager<tokio_postgres::NoTls>>;

pub fn build_pool(database_url: &str, sys: &mut actix_rt::SystemRunner) -> Pool {
    let bb8_manager = bb8_postgres::PostgresConnectionManager::new(
        database_url,
        tokio_postgres::NoTls,
    );

    use futures::lazy;
    sys.block_on(lazy(|| {
        bb8::Pool::builder()
            .max_size(12)
            .build(bb8_manager)
            .map_err(|e| bb8::RunError::User(e))
            .map_err(|e| panic!("{:?}", e))
    })).unwrap()
}

pub fn pool(
    pool: Data<Pool>
) -> impl Future<Item=HttpResponse, Error=Error> {
    test_pool(pool.get_ref())
        .from_err()
        .map(|t| HttpResponse::Ok().json(&t))
}

pub fn actor(
    db: Data<DB>
) -> impl Future<Item=HttpResponse, Error=Error> {
    db.send(GetTopics(vec![1u32, 2, 3, 4, 5, 6, 7, 8, 9, 10]))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(|(t, _)| HttpResponse::Ok().json(&t))
}

fn test_pool(
    pool: &Pool
) -> impl Future<Item=Vec<Topic>, Error=ResError> {
    pool.run(|mut c| {
        c.prepare("SELECT * FROM topics WHERE id = ANY($1)")
            .then(|res| match res {
                Ok(stmt) => Ok((stmt, c)),
                Err(e) => Err((e, c))
            })
            .and_then(|(stmt, mut c)| c
                .query(&stmt, &[&vec![1u32, 2, 3, 4, 5, 6, 7, 8, 9, 10]])
                .fold(Vec::with_capacity(10), move |mut v, r| {
                    if let Some(r) = Topic::try_from(r).ok() {
                        v.push(r)
                    }
                    Ok::<_, _>(v)
                })
                .then( |r| match r {
                    Ok(t) => Ok((t, c)),
                    Err(e) => Err((e, c))
                })
            )
    }).map_err(|_| ResError::InternalServerError)
}