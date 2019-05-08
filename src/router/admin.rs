use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, IntoFuture};

use crate::handler::auth::UserJwt;
use crate::model::{
    category::CategoryUpdateRequest,
    common::{PostgresPool, QueryOption, RedisPool},
    errors::ServiceError,
    post::PostRequest,
    topic::TopicRequest,
    user::{ToUserRef, UserUpdateRequest},
};
use crate::model::common::QueryOptAsync;
use crate::handler::cache::UpdateCache;
use core::borrow::Borrow;

/// Admin query will hit database directly.
pub fn admin_modify_category(jwt: UserJwt, req: Json<CategoryUpdateRequest>, cache: Data<RedisPool>, db: Data<PostgresPool>)
                             -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    req.to_privilege_check(&jwt.is_admin)
        .handle_check(&db)
        .into_future()
        .from_err()
        .and_then(move |_| match req.category_id {
            Some(_) => req
                .to_update_query()
                .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
                .into_future(),
            None => req
                .to_add_query()
                .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
                .into_future()
        })
}

pub fn admin_remove_category(jwt: UserJwt, id: Path<(u32)>, cache: Data<RedisPool>, db: Data<PostgresPool>)
                             -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    // ToDo: need to add posts and topics migration along side the remove.
    use crate::model::{admin::IdToQuery as AdminIdToQuery, category::IdToQuery};
    id.to_privilege_check(&jwt.is_admin)
        .handle_check(&db)
        .into_future()
        .from_err()
        .and_then(move |_| id
            .to_delete_query()
            .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
            .into_future())
}


pub fn admin_update_user(jwt: UserJwt, mut req: Json<UserUpdateRequest>, cache: Data<RedisPool>, db: Data<PostgresPool>)
                         -> impl Future<Item=HttpResponse, Error=Error> {
    req.attach_id_admin(None)
        .to_privilege_check(&jwt.is_admin)
        .handle_check(&db)
        .into_future()
        .from_err()
        .and_then(move |_| req
            .into_inner()
            .into_update_query()
            .into_user(QueryOptAsync::new(Some(db), None))
            .from_err()
            .and_then(move |u| {
                let _ignore = UpdateCache::GotUser(&u).handle_update(&Some(&cache));
                HttpResponse::Ok().json(u.to_ref())
            }))
}

pub fn admin_update_topic(jwt: UserJwt, mut req: Json<TopicRequest>, cache: Data<RedisPool>, db: Data<PostgresPool>)
                          -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    req.attach_user_id(None)
        .to_privilege_check(&jwt.is_admin)
        .handle_check(&db)
        .into_future()
        .from_err()
        .and_then(move |_| req
            .to_update_query()
            .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
            .into_future())
}

pub fn admin_update_post(jwt: UserJwt, mut req: Json<PostRequest>, cache: Data<RedisPool>, db: Data<PostgresPool>)
                         -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    req.attach_user_id(None)
        .to_privilege_check(&jwt.is_admin)
        .handle_check(&db)
        .into_future()
        .from_err()
        .and_then(move |_| req
            .to_update_query()
            .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
            .into_future())
}
