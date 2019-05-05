use futures::{Future, IntoFuture};

use actix_web::{web::{Data, Json, Path}, HttpResponse};

use crate::model::{
    errors::ServiceError,
    admin::AdminPrivilegeCheck,
    post::{PostRequest, PostQuery},
    topic::{TopicQuery, TopicRequest},
    category::{CategoryQuery, CategoryUpdateJson},
    user::{UserQuery, UserUpdateJson},
    common::{PostgresPool, RedisPool, QueryOption},
};
use crate::handler::auth::UserJwt;

// ToDo: Test update result.

/// Admin query will hit database directly.
pub fn admin_modify_category(jwt: UserJwt, req: Json<CategoryUpdateJson>, cache: Data<RedisPool>, db: Data<PostgresPool>)
                             -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    AdminPrivilegeCheck::UpdateCategoryCheck(&jwt.is_admin, &req.to_request())
        .handle_check(&db)
        .into_future()
        .from_err()
        .and_then(move |_| match req.category_id {
            Some(_) => CategoryQuery::UpdateCategory(&req.to_request())
                .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
                .into_future(),
            None => CategoryQuery::AddCategory(&req.to_request())
                .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
                .into_future()
        })
}

pub fn admin_remove_category(jwt: UserJwt, path: Path<(u32)>, cache: Data<RedisPool>, db: Data<PostgresPool>)
                             -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    // ToDo: need to add posts and topics migration along side the remove.
    AdminPrivilegeCheck::DeleteCategoryCheck(&jwt.is_admin)
        .handle_check(&db)
        .into_future()
        .from_err()
        .and_then(move |_| CategoryQuery::DeleteCategory(&path.as_ref())
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
