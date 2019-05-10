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


pub fn admin_update_user(
    jwt: UserJwt,
    mut req: Json<UpdateRequest>,
    cache: Data<RedisPool>,
    db: Data<PostgresPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    req.attach_id(None)
        .to_privilege_check(&jwt.is_admin)
        .handle_check(&db)
        .into_future()
        .from_err()
        .and_then(move |_| req
            .into_inner()
            .into_update_query()
            .into_user(db, None))
        .from_err()
        .and_then(move |u| {
            let res = HttpResponse::Ok().json(u.to_ref());
            UpdateCacheAsync::GotUser(u).handler(&cache).then(|_| res)
        })
}

pub fn admin_update_topic(
    jwt: UserJwt,
    mut req: Json<TopicRequest>,
    cache: Data<RedisPool>,
    db: Data<PostgresPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    req.attach_user_id(None)
        .to_privilege_check(&jwt.is_admin)
        .handle_check(&db)
        .into_future()
        .from_err()
        .and_then(move |_| req
            .into_inner()
            .into_update_query()
            .into_topics(&db))
        .from_err()
        .and_then(move |t| {
            let res = HttpResponse::Ok().json(&t);
            UpdateCacheAsync::GotTopics(t).handler(&cache).then(|_| res)
        })
}

pub fn admin_update_post(
    jwt: UserJwt,
    mut req: Json<PostRequest>,
    cache: Data<RedisPool>,
    db: Data<PostgresPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    req.attach_user_id(None)
        .to_privilege_check(&jwt.is_admin)
        .handle_check(&db)
        .into_future()
        .from_err()
        .and_then(move |_| req
            .into_inner()
            .into_update_query()
            .into_post(&db))
        .from_err()
        .and_then(move |p| {
            let res = HttpResponse::Ok().json(&p);
            UpdateCacheAsync::GotPost(p).handler(&cache).then(|_| res)
        })
}
