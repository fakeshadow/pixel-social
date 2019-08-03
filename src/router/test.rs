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
use crate::model::actors::DatabaseServiceRaw;

pub type Pool = l337::Pool<l337_postgres::PostgresConnectionManager<tokio_postgres::NoTls>>;

pub fn build_pool(sys: &mut actix_rt::SystemRunner) -> Pool {
    let config = tokio_postgres::Config::new()
        .host("localhost")
        .user("postgres")
        .password("123")
        .dbname("test").clone();

    let manager = l337_postgres::PostgresConnectionManager::new(
        config,
        tokio_postgres::NoTls,
    );

    let cfg = l337::Config {
        min_size: 1,
        max_size: 12,
    };

    sys.block_on(l337::Pool::new(manager, cfg)).unwrap()
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
    db.send(GetTopics(vec![1u32, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(|(t, _)| HttpResponse::Ok().json(&t))
}

pub fn raw(
    db: Data<DatabaseServiceRaw>
) -> impl Future<Item=HttpResponse, Error=Error> {
    db.test()
        .from_err()
        .and_then(|t| HttpResponse::Ok().json(&t))
}


impl DatabaseServiceRaw {
    fn test(&self) -> impl Future<Item=Vec<Topic>, Error=ResError> {

        match self.client.lock() {
            Ok(mut c) => futures::future::Either::B(c
                .query(&self.topics_by_id, &[&vec![1u32, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]])
                .fold(Vec::with_capacity(20), move |mut v, r| {
                    if let Some(r) = Topic::try_from(r).ok() {
                        v.push(r)
                    }
                    Ok::<_, _>(v)
                }).map_err(|_| ResError::InternalServerError)
            ),
            Err(_) => futures::future::Either::A(futures::future::err(ResError::InternalServerError))
        }
    }
}


fn test_pool(
    pool: &Pool
) -> impl Future<Item=Vec<Topic>, Error=ResError> {
    pool.connection()
        .map_err(|_| ResError::InternalServerError)
        .and_then(|mut c| {
            c.client
                .prepare("SELECT * FROM topics WHERE id = ANY($1)")
                .map_err(|_| ResError::InternalServerError)
                .and_then(move |stmt| {
                    c.client
                        .query(&stmt, &[&vec![1u32, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]])
                        .fold(Vec::with_capacity(20), move |mut v, r| {
                            if let Some(r) = Topic::try_from(r).ok() {
                                v.push(r)
                            }
                            Ok::<_, _>(v)
                        }).map_err(|_| ResError::InternalServerError)
                })
        })
}