use actix::prelude::Future;
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