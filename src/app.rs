use actix::prelude::*;
use actix_web::{http::Method, middleware, App};

use crate::model::db::DbExecutor;

use std::sync::{Arc, Mutex};

use crate::router::user;

pub struct AppState {
    pub db: Addr<DbExecutor>,
    pub next_ids: NextIds
}

#[derive(Clone, Debug)]
pub struct NextIds {
    pub next_uid: Arc<Mutex<u32>>,
    pub next_pid: Arc<Mutex<u32>>,
    pub next_tid: Arc<Mutex<u32>>
}

impl NextIds {
    pub fn create(vec: Vec<u32>) -> NextIds {
        NextIds {
            next_uid: Arc::new(Mutex::new(vec[0])),
            next_pid: Arc::new(Mutex::new(vec[1])),
            next_tid: Arc::new(Mutex::new(vec[2])),
        }
    }
}

pub fn create_app(db: Addr<DbExecutor>, next_ids: NextIds) -> App<AppState> {
    App::with_state(AppState { db, next_ids })
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


