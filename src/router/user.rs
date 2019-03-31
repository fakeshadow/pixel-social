use actix_web::{web, HttpResponse};

use futures::IntoFuture;

use crate::model::response::Response;
use crate::model::user::*;
use crate::model::types::*;

use crate::handler::auth::UserJwt;
use crate::model::common::Validator;
use crate::model::errors::ServiceError;
use crate::util::validation::validate_username;

use crate::handler::user::user_handler;



pub fn get_user(
    user_jwt: UserJwt,
    username: web::Path<String>,
    db: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {

    if !validate_username(&username) {return Err(ServiceError::UsernameShort)}

    let name = username.to_string();
    let user_query = if &name == "me" {
        UserQuery::GetMe(user_jwt.user_id)
    } else {
        UserQuery::GetUser(name)
    };

    match_query_result(user_handler(user_query, db))
}

pub fn login_user(
    login_request: web::Json<AuthRequest>,
    db: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {

    if !login_request.check_login() {return Err(ServiceError::BadRequestGeneral)}

    let user_query = UserQuery::Login(AuthRequest {
        username: login_request.username.clone(),
        password: login_request.password.clone(),
        email: None,
    });

    match_query_result(user_handler(user_query, db))
}

pub fn update_user(
    user_jwt: UserJwt,
    update_request: web::Json<UserUpdateRequest>,
    db: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {

    if let Some(_) = update_request.username {
        if !update_request.check_username() {
            return Err(ServiceError::UsernameShort)
        }
    }

    let user_query = UserQuery::UpdateUser(UserUpdateRequest {
        id: Some(user_jwt.user_id),
        username: update_request.username.clone(),
        password: None,
        email: None,
        avatar_url: update_request.avatar_url.clone(),
        signature: update_request.signature.clone(),
        is_admin: None,
        blocked: None,
    });

    match_query_result(user_handler(user_query, db))
}

//pub fn update_user_password((update_request, state): (Json<UserUpdateRequest>, State<AppState>))
//                            -> FutureResponse<HttpResponse> {
//    state.db
//        .send(UserQuery::UpdateUser(UserUpdateRequest {
//            id: None,
//            username: None,
//            password: None,
//            email: update_request.username.clone(),
//            avatar_url: None,
//            signature: None,
//            is_admin: None,
//            blocked: None,
//        }))
//        .from_err()
//        .and_then(|db_response| match db_response {
//            Ok(query_result) => Ok(Response::Modified(true).response()),
//            Err(service_error) => Ok(service_error.error_response()),
//        })
//        .responder()
//}

pub fn register_user(
    register_request: web::Json<AuthRequest>,
    db: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {

    if !register_request.check_register() { return Err(ServiceError::RegisterLimit)}

    let user_query = UserQuery::Register(AuthRequest {
            username: register_request.username.clone(),
            email: register_request.email.clone(),
            password: register_request.password.clone(),
        });

    match_query_result(user_handler(user_query, db))
}

//fn match_query_result(query_result: UserQueryResult) -> HttpResponse {
//    match query_result {
//        UserQueryResult::GotSlimUser(slim_user) => HttpResponse::Ok().json(slim_user),
//        UserQueryResult::GotUser(user) => HttpResponse::Ok().json(user),
//        UserQueryResult::LoggedIn(login_data) => HttpResponse::Ok().json(login_data),
//        UserQueryResult::Registered => Response::Register.response()
//    }
//}



fn match_query_result(result: Result<UserQueryResult, ServiceError>) -> Result<HttpResponse, ServiceError> {
    match result {
        Ok(query_result) => {
            match query_result {
                UserQueryResult::GotSlimUser(slim_user) => Ok(HttpResponse::Ok().json(slim_user)),
                UserQueryResult::GotUser(user) => Ok(HttpResponse::Ok().json(user)),
                UserQueryResult::LoggedIn(login_data) => Ok(HttpResponse::Ok().json(login_data)),
                UserQueryResult::Registered => Ok(Response::Register.response())
            }
        },
        Err(err) => Err(err)

    }
}

