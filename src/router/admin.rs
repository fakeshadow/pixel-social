use actix_web::{web, HttpResponse};
use futures::IntoFuture;

use crate::model::{
    errors::ServiceError,
    admin::*,
    common::{ResponseMessage, PostgresPool, RedisPool},
};

use crate::handler::{
    cache::cache_handler,
    category::category_handler,
};

pub fn admin_modify_category(
    admin_request: Json<AdminJson>,
    cache: web::Data<RedisPool>,
    db: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {

    match category_request.modify_type {
        Some(request_type) => if request_type > 2 {
           return Err(ServiceError::NotFound)
        } else {
            let admin_request = AdminRequest {
                modify_type: category_request.modify_type.clone(),
                category_id: category_request.category_id.clone(),
                category_data: category_request.category_data.clone(),

            };


        },
        None => Err(ServiceError::NotFound)
    }
}

pub fn admin_update_user(
cache: web::Data<RedisPool>,
db: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    state.db
        .send(UserQuery::UpdateUser(UserUpdateRequest {
            id: user_update_request.id.clone(),
            username: None,
            password: None,
            email: None,
            avatar_url: None,
            signature: None,
            is_admin: user_update_request.is_admin.clone(),
            blocked: user_update_request.blocked.clone(),
        }))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(_) => Ok(Response::Modified.response()),
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}

pub fn admin_update_topic(
    cache: web::Data<RedisPool>,
    db: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {

    state.db
        .send(TopicQuery::UpdateTopic(topic_update_request.clone()))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(_) => Ok(Response::Modified.response()),
            Err(service_error) => Ok(service_error.error_response())
        })
        .responder()
}

fn match_query_result(result: Result<AdminQueryResult, ServiceError>) -> Result<HttpResponse, ServiceError> {
    match result {
        Ok(query_result) => {
            match query_result {
                UserQueryResult::GotSlimUser(slim_user) => Ok(HttpResponse::Ok().json(slim_user)),
                UserQueryResult::GotUser(user) => Ok(HttpResponse::Ok().json(user)),
                UserQueryResult::LoggedIn(login_data) => Ok(HttpResponse::Ok().json(login_data)),
                UserQueryResult::Registered => Ok(HttpResponse::Ok().json(ResponseMessage::new("Register Success")))
            }
        },
        Err(err) => Err(err)

    }
}