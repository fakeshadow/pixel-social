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

use crate::handler::{
    admin::admin_handler,
    auth::UserJwt,
};

pub fn admin_modify_category(
    user_jwt: UserJwt,
    json: web::Json<CategoryUpdateJson>,
    cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);
    let request = json.to_request();

    // admin privilege check. need to improve for a complex level system.
    let admin_query = AdminQuery::UpdateCategoryCheck(&user_jwt.is_admin, &request);
    admin_handler(admin_query, &opt)?;

    let category_query = match request.category_id {
        Some(_category_id) => CategoryQuery::UpdateCategory(&request),
        None => CategoryQuery::AddCategory(&request)
    };
    Ok(category_query.handle_query(&opt)?.to_response())
}

pub fn admin_remove_category(
    user_jwt: UserJwt,
    remove_request: web::Path<(u32)>,
    cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    // need to add posts and topics migration along side the remove.
    let category_id = remove_request.as_ref();

    let opt = QueryOption::new(Some(&db_pool), None, None);

    let admin_query = AdminQuery::DeleteCategoryCheck(&user_jwt.is_admin, &category_id);
    admin_handler(admin_query, &opt)?;

    let category_query = CategoryQuery::DeleteCategory(&category_id);

    Ok(category_query.handle_query(&opt)?.to_response())
}

pub fn admin_update_user(
    user_jwt: UserJwt,
    json: web::Json<UserUpdateJson>,
    cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let id = match json.id {
        Some(id) => id,
        None => return Err(ServiceError::BadRequestGeneral)
    };

    let opt = QueryOption::new(Some(&db_pool), None, None);

    let update_request = json.to_request_admin(&id);

    //ToDo: impl trait for admin handler
    admin_handler(AdminQuery::UpdateUserCheck(&user_jwt.is_admin, &update_request), &opt)?;

    Ok(UserQuery::UpdateUser(&update_request).handle_query(&opt)?.to_response())
}

pub fn admin_update_topic(
    user_jwt: UserJwt,
    json: web::Json<TopicJson>,
    cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);
    let request = json.to_request(None);

    let admin_query = AdminQuery::UpdateTopicCheck(&user_jwt.is_admin, &request);
    admin_handler(admin_query, &opt)?;

    Ok(TopicQuery::UpdateTopic(&request).handle_query(&opt)?.to_response())
}

pub fn admin_update_post(
    user_jwt: UserJwt,
    json: web::Json<PostJson>,
//	cache_pool: web::Data<RedisPool>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let opt = QueryOption::new(Some(&db_pool), None, None);

    let post_request = json.to_request(None);

    let admin_query = AdminQuery::UpdatePostCheck(&user_jwt.is_admin, &post_request);
    admin_handler(admin_query, &opt)?;

    Ok(HttpResponse::Ok().finish())
}
