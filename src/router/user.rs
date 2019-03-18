use actix_web::{
    AsyncResponder,
    FutureResponse,
    HttpResponse,
    ResponseError,
    State,
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

use crate::model::user::{
    LoginData,
    RegisterData,
    RegisterCheck,
    IncomingRegister,
};

use crate::errors::ServiceError;

pub fn register_user((incoming_register, state): (Json<IncomingRegister>, State<AppState>))
                     -> FutureResponse<HttpResponse> {
    let register_check = register_check(&incoming_register, &state).wait();
    match register_check {
        Err(service_error) => Box::new(future_result(Ok(service_error.error_response()))),
        Ok(_) => {
            let uid: u32;
            let guard = state.next_ids.next_uid.lock();
            match guard {
                Ok(mut mutex_guard) => {
                    uid = *mutex_guard;
                    *mutex_guard += 1;
                }
                Err(_) => return Box::new(future_result(Ok(ServiceError::ArcLockError.error_response())))
            }

            let register_data = RegisterData {
                uid,
                username: incoming_register.username.clone(),
                email: incoming_register.email.clone(),
                password: incoming_register.password.clone(),
            };

            state.db.send(register_data)
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
                  -> FutureResponse<HttpResponse> {
    let login_data = LoginData {
        username: incoming_login.username.clone(),
        password: incoming_login.password.clone(),
    };

    state.db
        .send(login_data)
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(logged_in_response_data) => Ok(HttpResponse::Ok().json(logged_in_response_data)),
            Err(service_error) => Ok(service_error.error_response()),
        })
        .responder()
}

fn register_check(incoming_register: &Json<IncomingRegister>, state: &State<AppState>) -> impl Future<Item=(), Error=ServiceError> {
    let register_check = RegisterCheck {
        username: incoming_register.username.clone(),
        email: incoming_register.email.clone(),
    };
    state.db
        .send(register_check)
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(_) => future_ok(()),
            Err(service_error) => future_error(service_error)
        })
}