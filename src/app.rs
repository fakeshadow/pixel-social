use actix::prelude::*;
use actix_web::{http::Method, middleware, App};

use crate::model::db::DbExecutor;

use crate::router::{user, post};

pub struct AppState {
    pub db: Addr<DbExecutor>,
}

pub fn create_app(db: Addr<DbExecutor>) -> App<AppState> {
    App::with_state(AppState { db })
        .middleware(middleware::Logger::new("\"%r\" %s %b %Dms"))
        .scope("/user", |api| {
            api
                .resource("/register", |r| {
                    r.method(Method::POST).with(user::register_user);
                })
                .resource("/login", |r| {
                    r.method(Method::POST).with(user::login_user);
                })
                .resource("/{username}", |r| {
                    r.method(Method::GET).with(user::get_user);
                })
        })
        .scope("/post", |api| {
            api
                .resource("/", |r| {
                    r.method(Method::GET).with(post::get_post);
                })
                .resource("/", |r| {
                    r.method(Method::POST).with(post::add_post);
                })
        })
}


