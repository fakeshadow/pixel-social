use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, future::Either, IntoFuture};

use crate::handler::{auth::UserJwt, cache::UpdateCacheAsync};
use crate::model::{
    category::CategoryUpdateRequest,
    common::{PostgresPool, RedisPool},
    post::PostRequest,
    topic::TopicRequest,
    user::{ToUserRef, UpdateRequest},
};

/// Admin query will hit database directly.
pub fn admin_modify_category(
    jwt: UserJwt,
    req: Json<CategoryUpdateRequest>,
    cache: Data<RedisPool>,
    db: Data<PostgresPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    req.to_privilege_check(&jwt.is_admin)
        .handle_check(&db)
        .into_future()
        .from_err()
        .and_then(move |_| match req.category_id {
            Some(_) => Either::A(
                req.into_inner()
                    .into_update_query()
                    .into_categories(&db)
                    .from_err()
                    .and_then(move |c| {
                        let res = HttpResponse::Ok().json(&c);
                        UpdateCacheAsync::GotCategories(c)
                            .handler(&cache)
                            .then(|_| res)
                    })
            ),
            None => Either::B(
                req.into_inner()
                    .into_add_query()
                    .into_categories(&db)
                    .from_err()
                    .and_then(move |c| {
                        let res = HttpResponse::Ok().json(&c);
                        UpdateCacheAsync::AddedCategory(c)
                            .handler(&cache)
                            .then(|_| res)
                    })
            ),
        })
}

pub fn admin_remove_category(
    jwt: UserJwt, id: Path<(u32)>,
    cache: Data<RedisPool>,
    db: Data<PostgresPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    // ToDo: need to add posts and topics migration along side the remove.
    use crate::model::{admin::IdToQuery as AdminIdToQuery, category::IdToQuery};
    id.to_privilege_check(&jwt.is_admin)
        .handle_check(&db)
        .into_future()
        .from_err()
        .and_then(move |_| id
            .to_delete_query()
            .into_category_id(&db))
        .from_err()
        .and_then(move |id|
            UpdateCacheAsync::DeleteCategory(id)
                .handler(&cache)
                .then(move |_| HttpResponse::Ok().json(id)))
}

use crate::model::{
    actors::{DB, CACHE},
    common::Validator,
};
use crate::handler::{
    user::UpdateUser,
    topic::UpdateTopic,
    cache::UpdateCache,
    admin::{
        UpdateTopicCheck,
        UpdateUserCheck,
    },
};

pub fn update_user(
    jwt: UserJwt,
    mut req: Json<UpdateRequest>,
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
    mut req: Json<TopicRequest>,
    cache: Data<CACHE>,
    db: Data<DB>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = req.into_inner().attach_user_id_into(None);
    db.send(UpdateTopicCheck(jwt.is_admin, req))
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
            }))
}

//pub fn admin_update_post(
//    jwt: UserJwt,
//    mut req: Json<PostRequest>,
//    cache: Data<RedisPool>,
//    db: Data<PostgresPool>,
//) -> impl Future<Item=HttpResponse, Error=Error> {
//    req.attach_user_id(None)
//        .to_privilege_check(&jwt.is_admin)
//        .handle_check(&db)
//        .into_future()
//        .from_err()
//        .and_then(move |_| req
//            .into_inner()
//            .into_update_query()
//            .into_post(db.get_ref().clone()))
//        .from_err()
//        .and_then(move |p| {
//            let res = HttpResponse::Ok().json(&p);
//            UpdateCacheAsync::GotPost(p).handler(&cache).then(|_| res)
//        })
//}
