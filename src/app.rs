use actix::prelude::*;
use actix_web::{http::{header, Method}, middleware, middleware::cors::Cors, App, fs};

use crate::model::db::DbExecutor;
use crate::router::*;

pub struct AppState {
    pub db: Addr<DbExecutor>
}

pub fn create_app(db: Addr<DbExecutor>) -> App<AppState> {
    App::with_state(AppState { db })
        .middleware(middleware::Logger::new("\"%r\" %s %b %Dms"))
        .configure(|app| {
            Cors::for_app(app)
                .allowed_origin("http://localhost:8080")
                .allowed_methods(vec!["GET", "POST"])
                .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                .allowed_header(header::CONTENT_TYPE)
                .max_age(3600)
                .resource("/admin/category/", |r| {
                    r.method(Method::POST).with(admin::admin_modify_category);
                })
                .resource("/admin//user/", |r| {
                    r.method(Method::POST).with(admin::admin_update_user);
                })
                .resource("/admin//topic/", |r| {
                    r.method(Method::POST).with(admin::admin_update_topic);
                })
                .resource("user/register/", |r| {
                    r.method(Method::POST).with(user::register_user);
                })
                .resource("user/login/", |r| {
                    r.method(Method::POST).with(user::login_user);
                })
                .resource("user/update/", |r| {
                    r.method(Method::POST).with(user::update_user);
                })
                .resource("user/{username}", |r| {
                    r.method(Method::GET).with(user::get_user);
                })
                .resource("/post/", |r| {
                    r.method(Method::POST).with(post::add_post);
                })
                .resource("/post/edit/", |r| {
                    r.method(Method::POST).with(post::update_post);
                })
                .resource("/post/{pid}", |r| {
                    r.method(Method::GET).with(post::get_post);
                })
                .resource("/topic/", |r| {
                    r.method(Method::POST).with(topic::add_topic);
                })
                .resource("/topic/edit/", |r| {
                    r.method(Method::POST).with(topic::update_topic);
                })
                .resource("/topic/{topic_id}/{page}", |r| {
                    r.method(Method::GET).with(topic::get_topic);
                })
                .resource("/categories/", |r| {
                    r.method(Method::GET).with(category::get_all_categories);
                })
                .resource("/categories/", |r| {
                    r.method(Method::POST).with(category::get_categories);
                })
                .resource("/categories/popular/{page}", |r| {
                    r.method(Method::GET).with(category::get_popular);
                })
                .resource("/categories/{category_id}/{page}", |r| {
                    r.method(Method::GET).with(category::get_category);
                })
                .resource("/upload/", |r| {
                    r.method(Method::POST).with(stream::upload_file);
                })
                .register()
        })
        .handler(
            "/public",
            fs::StaticFiles::new("./public")
                .unwrap()
                .show_files_listing())
}


