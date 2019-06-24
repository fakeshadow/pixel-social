use futures::{Future, future::Either, IntoFuture};

use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};

use crate::model::{
    actors::{DB, CACHE},
    post::PostRequest,
    topic::TopicRequest,
    common::Validator,
    category::CategoryRequest,
    user::{ToUserRef, UpdateRequest},
};
use crate::handler::{
    auth::UserJwt,
    user::UpdateUser,
    category::{UpdateCategory, AddCategory, RemoveCategory},
    post::ModifyPost,
    topic::UpdateTopic,
    cache::{UpdateCache, AddedCategory, RemoveCategoryCache},
    admin::{
        UpdatePostCheck,
        UpdateCategoryCheck,
        UpdateTopicCheck,
        UpdateUserCheck,
    },
};
pub fn add_category(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    cache: Data<CACHE>,
    db: Data<DB>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = req.into_inner();
    req.check_new()
        .into_future()
        .from_err()
        .and_then(move |_| db
            .send(UpdateCategoryCheck(jwt.is_admin, req))
            .from_err()
            .and_then(|r| r)
            .from_err()
            .and_then(move |req|
                db.send(AddCategory(req))
                    .from_err()
                    .and_then(|r| r)
                    .from_err()
                    .and_then(move |c| {
                        let res = HttpResponse::Ok().json(&c);
                        let _ = cache.do_send(AddedCategory(c));
                        res
                    })))
}

pub fn update_category(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    cache: Data<CACHE>,
    db: Data<DB>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = req.into_inner();
    req.check_update()
        .into_future()
        .from_err()
        .and_then(move |_| db
            .send(UpdateCategoryCheck(jwt.is_admin, req))
            .from_err()
            .and_then(|r| r)
            .from_err()
            .and_then(move |req|
                db.send(UpdateCategory(req))
                    .from_err()
                    .and_then(|r| r)
                    .from_err()
                    .and_then(move |c| {
                        let res = HttpResponse::Ok().json(&c);
                        let _ = cache.do_send(UpdateCache::Category(c));
                        res
                    })))
}

pub fn remove_category(
    jwt: UserJwt,
    id: Path<(u32)>,
    cache: Data<CACHE>,
    db: Data<DB>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let id = id.into_inner();
    //ToDo: add admin check
    db.send(RemoveCategory(id))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(move |_| cache
            .send(RemoveCategoryCache(id))
            .from_err()
            // ToDo: add retry if the delete of categories failed.
            .and_then(|r| r)
            .from_err()
            .and_then(|_| HttpResponse::Ok().finish())
        )
}

pub fn update_user(
    jwt: UserJwt,
    req: Json<UpdateRequest>,
    cache: Data<CACHE>,
    db: Data<DB>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = req.into_inner().attach_id(None);
    req.check_update()
        .into_future()
        .from_err()
        .and_then(move |_| db
            .send(UpdateUserCheck(jwt.is_admin, req))
            .from_err()
            .and_then(|r| r)
            .from_err()
            .and_then(move |r| db
                .send(UpdateUser(r))
                .from_err()
                .and_then(|r| r)
                .from_err()
                .and_then(move |u| {
                    let res = HttpResponse::Ok().json(&u);
                    let _ = cache.do_send(UpdateCache::User(u));
                    res
                })))
}

pub fn update_topic(
    jwt: UserJwt,
    req: Json<TopicRequest>,
    cache: Data<CACHE>,
    db: Data<DB>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let mut req = req.into_inner().attach_user_id(None);
    req.check_update()
        .into_future()
        .from_err()
        .and_then(move |_| db
            .send(UpdateTopicCheck(jwt.is_admin, req))
            .from_err()
            .and_then(|r| r)
            .from_err()
            .and_then(move |r| db
                .send(UpdateTopic(r))
                .from_err()
                .and_then(|r| r)
                .from_err()
                .and_then(move |t| {
                    let res = HttpResponse::Ok().json(&t);
                    let _ = cache.do_send(UpdateCache::Topic(t));
                    res
                })))
}

pub fn update_post(
    jwt: UserJwt,
    req: Json<PostRequest>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let mut req = req.into_inner().attach_user_id(None);
    req.check_update()
        .into_future()
        .from_err()
        .and_then(move |_| db
            .send(UpdatePostCheck(jwt.is_admin, req))
            .from_err()
            .and_then(|r| r)
            .from_err()
            .and_then(move |r| db
                .send(ModifyPost(r, None))
                .from_err()
                .and_then(|r| r)
                .from_err()
                .and_then(move |p| {
                    let res = HttpResponse::Ok().json(&p);
                    let _ = cache.do_send(UpdateCache::Post(p));
                    res
                })))
}
