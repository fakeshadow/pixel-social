use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, IntoFuture};

use crate::handler::auth::UserJwt;
use crate::model::{
    category::CategoryUpdateRequest,
    common::{PostgresPool, QueryOption, RedisPool},
    post::PostRequest,
    topic::TopicRequest,
    user::{ToUserRef, UpdateRequest},
};
use crate::handler::cache::UpdateCache;

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
            Some(_) => req
                .into_inner()
                .into_update_query()
                .into_category(&db)
                .from_err(),
            None => req
                .into_inner()
                .into_add_query()
                .into_category(&db)
                .from_err()
        })
        .and_then(move |c| {
            let c = vec![c];
            let _ignore = UpdateCache::GotCategories(&c).handle_update(&Some(&cache));
            HttpResponse::Ok().json(&c)
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
        .and_then(move |id| {
            let _ignore = UpdateCache::DeleteCategory(&id).handle_update(&Some(&cache));
            HttpResponse::Ok().json(id)
        })
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
            let _ignore = UpdateCache::GotUser(&u).handle_update(&Some(&cache));
            HttpResponse::Ok().json(u.to_ref())
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
            .into_topic(&db, None))
        .from_err()
        .and_then(|r| HttpResponse::Ok().json(&r))
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
            let _ignore = UpdateCache::GotPost(&p).handle_update(&Some(&cache));
            HttpResponse::Ok().json(&p)
        })
}
