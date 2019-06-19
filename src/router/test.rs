use futures::{Future, future::{Either, IntoFuture, ok as ft_ok, err as ft_err}};

use actix_web::{Error, HttpResponse, ResponseError, web::{Data, Json, Path}};

use crate::model::{
    actors::{DatabaseService, DB, CACHE},
    user::{AuthRequest, UpdateRequest},
    common::{AttachUser, GlobalGuard, Validator},
    topic::{TopicRequest, TopicWithUser, TopicWithPost},
};
use crate::handler::{
    user::GetUsers,
    topic::AddTopic,
    cache::AddedTopic,
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
            let cid = t.category_id;
            let res = HttpResponse::Ok().json(&t);
            let _ = cache.do_send(AddedTopic(t, cid));
            res
        })
}