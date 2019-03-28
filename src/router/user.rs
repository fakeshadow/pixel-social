use actix_web::{AsyncResponder, FutureResponse, HttpResponse, ResponseError, State, Json, Path};
use futures::{Future, future::result as future_result};

use crate::app::AppState;
use crate::model::response::Response;
use crate::model::user::*;
use crate::handler::auth::UserJwt;
use crate::model::common::Validator;
use crate::model::errors::ServiceError;
use crate::util::validation::validate_username;

pub fn get_user((username, user_jwt, state): (Path<String>, UserJwt, State<AppState>))
                -> FutureResponse<HttpResponse> {

    if !validate_username(&username) {
        return Box::new(future_result(Ok(ServiceError::BadRequestGeneral.error_response())));
    }

    let name = username.to_string();
    let message = if &name == "me" {
        UserQuery::GetMe(user_jwt.user_id)
    } else {
        UserQuery::GetUser(name)
    };

    state.db
        .send(message)
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(query_result) => Ok(match_query_result(query_result)),
            Err(service_error) => Ok(service_error.error_response()),
        })
        .responder()
}

pub fn login_user((login_request, state): (Json<AuthRequest>, State<AppState>))
                  -> FutureResponse<HttpResponse> {
    if login_request.check_login() == false {
        return Box::new(future_result(Ok(ServiceError::BadRequestGeneral.error_response())));
    }

    state.db
        .send(UserQuery::Login(AuthRequest {
            username: login_request.username.clone(),
            password: login_request.password.clone(),
            email: None,
        }))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(query_result) => Ok(match_query_result(query_result)),
            Err(service_error) => Ok(service_error.error_response()),
        })
        .responder()
}

pub fn update_user((update_request, user_jwt, state): (Json<UserUpdateRequest>, UserJwt, State<AppState>))
                   -> FutureResponse<HttpResponse> {
    if let Some(_) = update_request.username {
        if !update_request.check_username() {
            return Box::new(future_result(Ok(ServiceError::UsernameShort.error_response())));
        }
    }

    state.db
        .send(UserQuery::UpdateUser(UserUpdateRequest {
            id: Some(user_jwt.user_id),
            username: update_request.username.clone(),
            password: None,
            email: None,
            avatar_url: update_request.avatar_url.clone(),
            signature: update_request.signature.clone(),
            is_admin: None,
            blocked: None,
        }))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(query_result) => Ok(match_query_result(query_result)),
            Err(service_error) => Ok(service_error.error_response()),
        })
        .responder()
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

pub fn register_user((register_request, state): (Json<AuthRequest>, State<AppState>))
                     -> FutureResponse<HttpResponse> {
    if register_request.check_register() == false {
        return Box::new(future_result(Ok(ServiceError::BadRequestGeneral.error_response())));
    }

    state.db
        .send(UserQuery::Register(AuthRequest {
            username: register_request.username.clone(),
            email: register_request.email.clone(),
            password: register_request.password.clone(),
        }))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(query_result) => Ok(match_query_result(query_result)),
            Err(service_error) => Ok(service_error.error_response()),
        })
        .responder()
}

fn match_query_result(result: UserQueryResult) -> HttpResponse{
    match result {
        UserQueryResult::GotSlimUser(slim_user) => Response::SendData(slim_user).response(),
        UserQueryResult::GotUser(user) => Response::SendData(user).response(),
        UserQueryResult::LoggedIn(login_data) => Response::SendData(login_data).response(),
        UserQueryResult::Registered => Response::Register(true).response()
    }
}