use futures::IntoFuture;
use actix_web::{web, HttpResponse};

use crate::model::{
    errors::ServiceError,
    admin::AdminQuery,
    post::{PostJson, PostQuery},
    topic::{TopicQuery, TopicJson},
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
    Ok(query.handle_query(&opt)?.to_response())
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
    Ok(CategoryQuery::DeleteCategory(&category_id).handle_query(&opt)?.to_response())
}

pub fn admin_update_user(
    jwt: UserJwt,
    json: web::Json<UserUpdateJson>,
    cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let id = json.id.ok_or(ServiceError::BadRequestGeneral)?;
    let opt = QueryOption::new(Some(&db_pool), None, None);
    let update_request = json.to_request_admin(&id);

    AdminQuery::UpdateUserCheck(&jwt.is_admin, &update_request).handle_query(&opt)?;
    Ok(UserQuery::UpdateUser(&update_request).handle_query(&opt)?.to_response())
}

pub fn admin_update_topic(
    jwt: UserJwt,
    json: web::Json<TopicJson>,
    cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);
    let request = json.to_request(None);

    AdminQuery::UpdateTopicCheck(&jwt.is_admin, &request).handle_query(&opt)?;
    Ok(TopicQuery::UpdateTopic(&request).handle_query(&opt)?.to_response())
}

pub fn admin_update_post(
    jwt: UserJwt,
    json: web::Json<PostJson>,
//	cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);
    let req = json.to_request(None);

    AdminQuery::UpdatePostCheck(&jwt.is_admin, &req).handle_query(&opt)?;
    Ok(HttpResponse::Ok().finish())
}
