use futures::{Future, IntoFuture};

use actix_web::{web::{Data, Json, Path}, HttpResponse};

use crate::model::{
    errors::ServiceError,
    post::PostRequest,
    topic::TopicRequest,
    category::CategoryUpdateRequest,
    user::UserUpdateJson,
    common::{PostgresPool, RedisPool, QueryOption},
};
use crate::handler::auth::UserJwt;

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

pub fn admin_update_user(jwt: UserJwt, req: Json<UserUpdateJson>, cache: Data<RedisPool>, db: Data<PostgresPool>)
                         -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    req.to_request_admin()
        .to_privilege_check(&jwt.is_admin)
        .handle_check(&db)
        .into_future()
        .from_err()
        .and_then(move |_| req
            .to_request_admin()
            .to_update_query()
            .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
            .into_future())
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
