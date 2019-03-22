use actix::prelude::*;
use actix_web::{http::Method, middleware, App};

use crate::model::db::DbExecutor;
use crate::router::*;

pub struct AppState {
    pub db: Addr<DbExecutor>,
}

pub fn create_app(db: Addr<DbExecutor>) -> App<AppState> {
    App::with_state(AppState { db })
        .middleware(middleware::Logger::new("\"%r\" %s %b %Dms"))
        .scope("/admin", |api| {
            api
                .resource("/category/", |r| {
                    r.method(Method::POST).with(admin::admin_modify_category);
                })
                .resource("/user/", |r| {
                    r.method(Method::POST).with(admin::admin_update_user);
                })
                .resource("/topic/", |r| {
                    r.method(Method::POST).with(admin::admin_update_topic);
                })
//                .resource("/post/", |r| {
//                    r.method(Method::POST).with(user::register_user);
//                })
        })
        .scope("/user", |api| {
            api
                .resource("/register/", |r| {
                    r.method(Method::POST).with(user::register_user);
                })
                .resource("/login/", |r| {
                    r.method(Method::POST).with(user::login_user);
                })
                .resource("/update/", |r| {
                    r.method(Method::POST).with(user::update_user);
                })
                .resource("/{username}", |r| {
                    r.method(Method::GET).with(user::get_user);
                })
//                .resource("/block/", |r| {
//                    r.method(Method::POST).with(user::block_user);
//                })
        })
        .scope("/post", |api| {
            api
                .resource("/", |r| {
                    r.method(Method::POST).with(post::add_post);
                })
                .resource("/{pid}", |r| {
                    r.method(Method::GET).with(post::get_post);
                })
        })
        .scope("/topic", |api| {
            api
                .resource("/", |r| {
                    r.method(Method::POST).with(topic::add_topic);
                })
                .resource("/edit/", |r| {
                    r.method(Method::POST).with(topic::update_topic);
                })
                .resource("/{topic_id}/{page}", |r| {
                    r.method(Method::GET).with(topic::get_topic);
                })

        })
        .scope("/categories", |api| {
            api
                .resource("/", |r| {
                    r.method(Method::GET).with(category::get_all_categories);
                })
                .resource("/", |r| {
                    r.method(Method::POST).with(category::get_categories);
                })
                .resource("/popular/{page}", |r| {
                    r.method(Method::GET).with(category::get_popular);
                })
                .resource("/{category_id}/{page}", |r| {
                    r.method(Method::GET).with(category::get_category);
                })
        })
}


