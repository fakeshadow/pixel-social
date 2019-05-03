use futures::{Future, IntoFuture};

use actix_web::{web::{Data, Json, Path}, HttpResponse};

use crate::model::{
    errors::ServiceError,
    admin::AdminQuery,
    post::{PostRequest, PostQuery},
    topic::{TopicQuery, TopicRequest},
    category::{CategoryQuery, CategoryUpdateJson},
    user::{UserQuery, UserUpdateJson},
    common::{PostgresPool, RedisPool, QueryOption},
};
use crate::handler::auth::UserJwt;

// ToDo: Test update result.
pub fn admin_modify_category(jwt: UserJwt, req: Json<CategoryUpdateJson>, cache: Data<RedisPool>, db: Data<PostgresPool>)
                             -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    AdminQuery::UpdateCategoryCheck(&jwt.is_admin, &req.to_request())
        .handle_query(&db)
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
    AdminQuery::DeleteCategoryCheck(&jwt.is_admin)
        .handle_query(&db)
        .into_future()
        .from_err()
        .and_then(move |_| CategoryQuery::DeleteCategory(&path.as_ref())
            .handle_query(&QueryOption::new(Some(&db), Some(&cache), None)))
}

pub fn admin_update_user(jwt: UserJwt, req: Json<UserUpdateJson>, cache: Data<RedisPool>, db: Data<PostgresPool>)
                         -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    AdminQuery::UpdateUserCheck(&jwt.is_admin, &req.to_request_admin(&req.id.unwrap()))
        .handle_query(&db)
        .into_future()
        .from_err()
        .and_then(move |_| UserQuery::UpdateUser(&req.to_request_admin(&req.id.unwrap()))
            .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
            .into_future())
}

pub fn admin_update_topic(jwt: UserJwt, req: Json<TopicRequest>, cache: Data<RedisPool>, db: Data<PostgresPool>)
                          -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let req = req.into_inner().attach_user_id(None);
    AdminQuery::UpdateTopicCheck(&jwt.is_admin, &req)
        .handle_query(&db)
        .into_future()
        .from_err()
        .and_then(move |_| TopicQuery::UpdateTopic(req)
            .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
            .into_future())
}

pub fn admin_update_post(jwt: UserJwt, req: Json<PostRequest>, cache: Data<RedisPool>, db: Data<PostgresPool>)
                         -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let req = req.into_inner().attach_user_id(None);
    AdminQuery::UpdatePostCheck(&jwt.is_admin, &req)
        .handle_query(&db)
        .into_future()
        .from_err()
        .and_then(move |_| PostQuery::UpdatePost(&req)
            .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
            .into_future())
}
