use futures::IntoFuture;
use actix_web::{web, HttpResponse};

use crate::model::{
    errors::ServiceError,
    admin::AdminQuery,
    post::{PostRequest, PostQuery},
    topic::{TopicQuery, TopicRequest},
    category::{CategoryQuery, CategoryUpdateJson},
    user::{UserQuery, UserUpdateJson},
    common::{ResponseMessage, PostgresPool, RedisPool, QueryOption},
};
use crate::handler::auth::UserJwt;

// ToDo: Add more admin check.
pub fn admin_modify_category(
    jwt: UserJwt,
    json: web::Json<CategoryUpdateJson>,
    cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);
    let req = json.to_request();

    AdminQuery::UpdateCategoryCheck(&jwt.is_admin, &req).handle_query(&opt)?;

    let query = match req.category_id {
        Some(_category_id) => CategoryQuery::UpdateCategory(&req),
        None => CategoryQuery::AddCategory(&req)
    };
    query.handle_query(&opt)
}

pub fn admin_remove_category(
    jwt: UserJwt,
    remove_request: web::Path<(u32)>,
    cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    // ToDo: need to add posts and topics migration along side the remove.
    let opt = QueryOption::new(Some(&db_pool), None, None);
    let category_id = remove_request.as_ref();

    AdminQuery::DeleteCategoryCheck(&jwt.is_admin).handle_query(&opt)?;
    CategoryQuery::DeleteCategory(&category_id).handle_query(&opt)
}

pub fn admin_update_user(
    jwt: UserJwt,
    req: web::Json<UserUpdateJson>,
    cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let id = req.id.ok_or(ServiceError::BadRequestGeneral)?;
    let opt = QueryOption::new(Some(&db_pool), None, None);
    let update_request = req.to_request_admin(&id);

    AdminQuery::UpdateUserCheck(&jwt.is_admin, &update_request).handle_query(&opt)?;
    UserQuery::UpdateUser(&update_request).handle_query(&opt)
}

pub fn admin_update_topic(
    jwt: UserJwt,
    req: web::Json<TopicRequest>,
    cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);
    let request = req.into_inner().attach_user_id(None);

    AdminQuery::UpdateTopicCheck(&jwt.is_admin, &request).handle_query(&opt)?;
    TopicQuery::UpdateTopic(request).handle_query(&opt)
}

pub fn admin_update_post(
    jwt: UserJwt,
    req: web::Json<PostRequest>,
//	cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);

    AdminQuery::UpdatePostCheck(&jwt.is_admin, &req.into_inner().attach_user_id(None)).handle_query(&opt)?;
    Ok(HttpResponse::Ok().finish())
}
