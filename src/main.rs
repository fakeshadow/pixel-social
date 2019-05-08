#![allow(unused_imports)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

use std::env;

use actix::prelude::*;
use actix_files as fs;
use actix_web::{
    App,
    http::header,
    HttpServer, middleware::{cors::Cors, Logger}, web,
};
use diesel::{PgConnection, r2d2::ConnectionManager};
use dotenv::dotenv;
use r2d2_redis::{r2d2 as redis_r2d2, RedisConnectionManager};

use crate::handler::cache::clear_cache;
use crate::util::startup::{build_cache, init_global_var};

mod handler;
mod model;
mod router;
mod schema;
mod util;

fn main() -> std::io::Result<()> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = env::var("REDIS_URL").unwrap_or("redis://127.0.0.1".to_string());
    let server_ip = env::var("SERVER_IP").unwrap_or("127.0.0.1".to_string());
    let server_port = env::var("SERVER_PORT").unwrap_or("8081".to_string());
    let cors_origin = env::var("CORS_ORIGIN").unwrap_or("*".to_string());

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let postgres_pool = r2d2::Pool::builder()
        .max_size(12)
        .build(manager)
        .expect("Failed to create postgres pool.");

    let cache_manager = RedisConnectionManager::new(redis_url.as_str()).unwrap();
    let redis_pool = redis_r2d2::Pool::builder()
        .max_size(12)
        .build(cache_manager)
        .expect("Failed to create redis pool.");
    let _clear = clear_cache(&redis_pool);
    let _build = build_cache(&postgres_pool, &redis_pool);

    let global_arc = init_global_var(&postgres_pool);

    HttpServer::new(move || {
        App::new()
            .data(postgres_pool.clone())
            .data(redis_pool.clone())
            .data(global_arc.clone())
            .wrap(Logger::default())
            .wrap(Cors::new()
                .allowed_origin(&cors_origin)
                .allowed_methods(vec!["GET", "POST"])
                .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                .allowed_header(header::CONTENT_TYPE)
                .max_age(3600))
            .service(web::scope("/admin")
                .service(web::resource("/user").route(web::post().to_async(router::admin::admin_update_user)))
                .service(web::resource("/post").route(web::post().to_async(router::admin::admin_update_post)))
                .service(web::resource("/topic").route(web::post().to_async(router::admin::admin_update_topic)))
                .service(web::scope("/category")
                    .service(web::resource("/delete/{category_id}").route(web::get().to_async(router::admin::admin_remove_category)))
                    .service(web::resource("/add").route(web::post().to_async(router::admin::admin_modify_category)))
                    .service(web::resource("/update").route(web::post().to_async(router::admin::admin_modify_category)))))
            .service(web::scope("/user")
                .service(web::resource("/register").route(web::post().to_async(router::user::register_user)))
                .service(web::resource("/login").route(web::post().to_async(router::user::login_user)))
                .service(web::resource("/{id}").route(web::get().to_async(router::user::get_user)))
                .service(web::resource("/").route(web::post().to_async(router::user::update_user))))
            .service(web::scope("/post")
                .service(web::resource("/").route(web::post().to_async(router::post::add_post)))
                .service(web::resource("/{pid}").route(web::get().to_async(router::post::get_post)))
                .service(web::resource("/edit").route(web::post().to_async(router::post::update_post))))
            .service(web::scope("/topic")
                .service(web::resource("/").route(web::post().to_async(router::topic::add_topic)))
                .service(web::resource("/edit").route(web::post().to_async(router::topic::update_topic)))
                .service(web::resource("/{topic_id}/{page}").route(web::get().to_async(router::topic::get_topic))))
            .service(web::scope("/categories")
                .service(web::resource("/")
                    .route(web::get().to_async(router::category::get_all_categories))
                    .route(web::post().to_async(router::category::get_categories)))
                .service(web::resource("/popular/{page}").route(web::get().to_async(router::category::get_popular)))
                .service(web::resource("/{category_id}/{page}").route(web::get().to_async(router::category::get_category))))
            .service(web::scope("/test")
                .service(web::resource("/lock").route(web::get().to_async(router::test::test_global_var))))
            .service(web::scope("/upload")
                .service(web::resource("/").route(web::post().to_async(router::stream::upload_file))))
            .service(fs::Files::new("/public", "./public"))
    }).bind(format!("{}:{}", &server_ip, &server_port))?.run()
}
