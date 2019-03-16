use actix::prelude::*;
use actix_web::{http::Method, middleware, App, HttpRequest};

use crate::DbExecutor;

use std::sync::{Arc, Mutex};

use crate::router::user;

pub struct AppState {
    pub db: Addr<DbExecutor>,
    pub next_uid: Arc<Mutex<u32>>,
    pub next_pid: Arc<Mutex<u32>>,
    pub next_tid: Arc<Mutex<u32>>,
}

pub fn create_app(db: Addr<DbExecutor>, next_uid: Arc<Mutex<u32>>, next_pid: Arc<Mutex<u32>>, next_tid: Arc<Mutex<u32>>) -> App<AppState> {
    App::with_state(AppState { db, next_uid, next_pid, next_tid })
        .middleware(middleware::Logger::new("\"%r\" %s %b %Dms"))
        .scope("/user", |api| {
            api
                .resource("/register", |r| {
                    r.method(Method::POST).with(user::register_user);
                })
                .resource("/login", |r| {
                    r.method(Method::POST).with(user::login_user);
                })
        })
}

