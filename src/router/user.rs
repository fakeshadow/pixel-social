use actix_web::{AsyncResponder, FutureResponse, HttpResponse, ResponseError, State, Json, Path};

use futures::Future;

use crate::app::AppState;
use crate::model::response::Response;
use crate::model::user::{UserQuery, RegisterRequest, LoginRequest};
use crate::handler::auth::UserJwt;

pub fn register_user((register_request, state): (Json<RegisterRequest>, State<AppState>))
                     -> FutureResponse<HttpResponse> {
    state.db
        .send(UserQuery::Register(RegisterRequest {
            username: register_request.username.clone(),
            email: register_request.email.clone(),
            password: register_request.password.clone(),
        }))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(_) => Ok(Response::RegisterSuccess(true).response()),
            Err(service_error) => Ok(service_error.error_response()),
        })
        .responder()
}

pub fn login_user((login_request, state): (Json<LoginRequest>, State<AppState>))
                  -> FutureResponse<HttpResponse> {
    state.db
        .send(UserQuery::Login(LoginRequest {
            username: login_request.username.clone(),
            password: login_request.password.clone(),
        }))
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(query_result) => {
                match query_result.to_login_data() {
                    Some(login_data) => Ok(Response::LoginSuccess(login_data).response()),
                    None => Ok(Response::ToError(true).response())
                }
            },
            Err(service_error) => Ok(service_error.error_response()),
        })
        .responder()
}

pub fn get_user((username, user_jwt, state): (Path<String>, UserJwt, State<AppState>))
                -> FutureResponse<HttpResponse> {
    // add check username here

    let name = username.to_string();
    let message = if &name == "me" {
        UserQuery::GetMe(user_jwt.uid)
    } else {
        UserQuery::GetUser(name)
    };

    state.db
        .send(message)
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(query_result) => {
                match query_result.to_user_data() {
                    None => Ok(Response::ToError(true).response()),
                    Some(user_data) => Ok(Response::GetUserSuccess(user_data).response())
                }
            },
            Err(service_error) => Ok(service_error.error_response()),
        })
        .responder()
}
