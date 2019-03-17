use actix_web::{
    AsyncResponder,
    FutureResponse,
    HttpResponse,
    ResponseError,
    State,
    Error,
    Json};

use futures::{
    future::{
        err as future_error,
        result as future_result,
        ok as future_ok,
    },
    Future,
};

use crate::app::AppState;

use crate::handler::{
    login::LoginData,
    register::{
        RegisterData,
        RegisterCheck,
        IncomingRegister,
    },
};

use crate::errors::ServiceError;
use diesel::query_dsl::RunQueryDsl;

use std::fmt::Display;


pub fn register_user((incoming_register, state): (Json<IncomingRegister>, State<AppState>))
                     -> FutureResponse<HttpResponse> {
    let register_check = register_check(&incoming_register, &state).wait();
    match register_check {
        Err(service_error) => Box::new(future_result(Ok(service_error.error_response()))),
        Ok(exist) => {
            let mut uid: u32 = 1;
            let guard = state.next_uid.lock();
            match guard {
                Ok(mut mutex_guard) => {
                    uid = *mutex_guard;
                    *mutex_guard += 1;
                }
                Err(_) => {}
            }

            let msg = RegisterData {
                uid,
                username: incoming_register.username.clone(),
                email: incoming_register.email.clone(),
                password: incoming_register.password.clone(),
            };

            state.db.send(msg)
                .from_err()
                .and_then(|db_response| match db_response {
                    Ok(_) => Ok(HttpResponse::Ok().json("Register Success")),
                    Err(service_error) => Ok(service_error.error_response()),
                })
                .responder()
        }
    }
}

pub fn login_user((incoming_login, state): (Json<LoginData>, State<AppState>))
                  -> HttpResponse {
    HttpResponse::Ok().json("abc")
}

fn register_check(incoming_register: &Json<IncomingRegister>, state: &State<AppState>) -> impl Future<Item=bool, Error=ServiceError> {
    let register_check = RegisterCheck {
        username: incoming_register.username.clone(),
        email: incoming_register.email.clone(),
    };
    state.db
        .send(register_check)
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(exist) => future_ok(exist),
            Err(service_error) => future_error(service_error)
        })
}