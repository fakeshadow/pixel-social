use futures::Future;

use actix_web::{Error, HttpResponse, ResponseError, web::{Data, Json, Path}};

use crate::model::{
    common::{GlobalGuard, PostgresPool, RedisPool},
    topic::{TopicRequest, TopicQuery},
};
use crate::handler::cache::UpdateCacheAsync;

pub fn test_global_var(
    global: Data<GlobalGuard>,
    db: Data<PostgresPool>,
    cache: Data<RedisPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    TopicQuery::AddTopic(TopicRequest {
        id: None,
        user_id: Some(1),
        category_id: Some(1),
        thumbnail: Some("test thumbnail".to_string()),
        title: Some("test title".to_string()),
        body: Some("test body".to_string()),
        is_locked: None,
    }).into_topic_with_category(db.get_ref().clone(), Some(global.get_ref().clone()))
        .from_err()
        .and_then(move |(c, t)|
            UpdateCacheAsync::AddedTopic(c, t)
                .handler(&cache)
                .then(|_| HttpResponse::Ok().finish()))
}

use crate::handler::db::{PostgresConnection, DB, GetCategories, GetTopics, GetTopic, GetUsers, GetPosts, AddTopic};
use crate::model::common::AttachUser;
use crate::model::topic::{TopicWithUser, TopicWithPost};
use crate::handler::auth::UserJwt;

pub fn get_all_categories(db: Data<DB>) -> impl Future<Item=HttpResponse, Error=Error> {
    db.send(GetCategories)
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(|c| HttpResponse::Ok().json(c))
}

pub fn get_category(
    req: Path<(u32, i64)>,
    db: Data<DB>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let (id, page) = req.into_inner();
    db.send(GetTopics(vec![id], page))
        .from_err()
        .and_then(|r| r)
        .from_err()
        // return user ids with topics for users query
        .and_then(move |(t, ids)|
            db.send(GetUsers(ids))
                .from_err()
                .and_then(|r| r)
                .from_err()
                .and_then(move |u| HttpResponse::Ok().json(&t.iter()
                    .map(|t| t.attach_user(&u))
                    .collect::<Vec<TopicWithUser>>())))
}

pub fn get_topic(
    req: Path<(u32, i64)>,
    db: Data<DB>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let (id, page) = req.into_inner();
    db.send(GetTopic(id))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(move |t| db
            .send(GetPosts(t.id, page))
            .from_err()
            .and_then(|r| r)
            .from_err()
            // return user ids and posts for users query
            .and_then(move |(p, mut ids)| {
                // push topic's user_id and sort user ids
                ids.push(t.user_id);
                ids.sort();
                ids.dedup();
                db.send(GetUsers(ids))
                    .from_err()
                    .and_then(|r| r)
                    .from_err()
                    .and_then(move |u| {
                        // include topic when querying first page.
                        let topic = if page == 1 { Some(&t) } else { None };
                        HttpResponse::Ok().json(TopicWithPost::new(topic, &p, &u))
                    })
            }))
}

pub fn add_topic(
    jwt: UserJwt,
    db: Data<DB>,
    req: Json<TopicRequest>,
    global: Data<GlobalGuard>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = req.into_inner().attach_user_id_into(Some(jwt.user_id));
    db.send(AddTopic(req, global.get_ref().clone()))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(|r| HttpResponse::Ok().json("test"))
}