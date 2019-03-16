use actix_web::{AsyncResponder, FutureResponse, HttpResponse, ResponseError, State, Json};
use futures::{future::result as err_result, future::Future};

use crate::app::AppState;

use crate::handler::register::{RegisterData, IncomingRegister};
use crate::handler::login::LoginData;

pub fn register_user((incoming_register, state): (Json<IncomingRegister>, State<AppState>))
                     -> FutureResponse<HttpResponse> {
    let mut uid: u32 = 1;
    let guard = state.next_uid.lock();
    match guard {
        Ok(mut mutex_guard) => {
            uid = *mutex_guard;
            *mutex_guard += 1;
        },
        Err(err) => {
            return Box::new(err_result(Ok(HttpResponse::Ok().json("Register too busy"))))
        }
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

pub fn login_user((incoming_login, state): (Json<LoginData>, State<AppState>))
                  -> HttpResponse {
    HttpResponse::Ok().json("abc")
}