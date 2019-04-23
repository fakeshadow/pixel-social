#![allow(unused_imports)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;

use std::env;

use actix::prelude::*;
use actix_files as fs;
use actix_web::{
    http::header,
    middleware::{cors::Cors, Logger},
    web, App, HttpServer,
};
use dotenv::dotenv;
use diesel::{r2d2::ConnectionManager, PgConnection};
use r2d2_redis::{r2d2 as redis_r2d2, redis, RedisConnectionManager};

mod handler;
mod model;
mod router;
mod schema;
mod util;

use crate::model::common::GlobalVar;

fn main() -> std::io::Result<()> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
    let server_ip = env::var("SERVER_IP").unwrap_or("127.0.0.1".to_string());
    let server_port = env::var("SERVER_PORT").unwrap_or("8081".to_string());
    let cors_origin = env::var("CORS_ORIGIN").unwrap_or("*".to_string());

    //     clear cache on start up for test purpose
    let redis_client = r2d2_redis::redis::Client::open(redis_url.as_str()).unwrap();
    let clear_cache = redis_client.get_connection().unwrap();
    let _result: Result<usize, _> = redis::cmd("flushall").query(&clear_cache);

    let global_arc = GlobalVar::init(&database_url);

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let postgres_pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create postgres pool.");

    let cache_manager = RedisConnectionManager::new(redis_url.as_str()).unwrap();
    let redis_pool = redis_r2d2::Pool::builder()
        .build(cache_manager)
        .expect("Failed to create redis pool.");

    let sys = actix_rt::System::new("PixelShare");

    HttpServer::new(move || {
        App::new()
            .data(postgres_pool.clone())
            .data(redis_pool.clone())
            .data(global_arc.clone())
            .wrap(Logger::default())
            .wrap(
                Cors::new()
                    .allowed_origin(&cors_origin)
                    .allowed_methods(vec!["GET", "POST"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .max_age(3600),
            )
            .service(
                web::scope("/admin")
                    .service(
                        web::resource("/user").route(web::post().to_async(router::admin::admin_update_user)),
                    )
                    .service(
                        web::resource("/post").route(web::post().to_async(router::admin::admin_update_post)),
                    )
                    .service(
                        web::resource("/topic").route(web::post().to_async(router::admin::admin_update_topic)),
                    )
                    .service(
                        web::scope("/category")
                            .service(
                                web::resource("/delete/{category_id}").route(web::get().to_async(router::admin::admin_remove_category)),
                            )
                            .service(
                                web::resource("/add").route(web::post().to_async(router::admin::admin_modify_category)),
                            )
                            .service(
                                web::resource("/update").route(web::post().to_async(router::admin::admin_modify_category)),
                            )
                    )
            )
            .service(
                web::scope("/user")
                    .service(
                        web::resource("/")
                            .route(web::post().to_async(router::user::update_user))
                            .route(web::get().to_async(router::user::get_user)),
                    )
                    .service(
                        web::resource("/register")
                            .route(web::post().to_async(router::user::register_user)),
                    )
                    .service(
                        web::resource("/login")
                            .route(web::post().to_async(router::user::login_user)),
                    ),
            )
            .service(
                web::scope("/post")
                    .service(web::resource("/").route(web::post().to_async(router::post::add_post)))
                    .service(
                        web::resource("/{pid}").route(web::get().to_async(router::post::get_post)),
                    )
                    .service(
                        web::resource("/edit")
                            .route(web::post().to_async(router::post::update_post)),
                    ),
            )
            .service(
                web::scope("/topic")
                    .service(
                        web::resource("/").route(web::post().to_async(router::topic::add_topic)),
                    )
                    .service(
                        web::resource("/edit")
                            .route(web::post().to_async(router::topic::update_topic)),
                    )
                    .service(
                        web::resource("/{topic_id}/{page}")
                            .route(web::get().to_async(router::topic::get_topic)),
                    ),
            )
            .service(
                web::scope("/categories")
                    .service(
                        web::resource("/")
                            .route(web::get().to_async(router::category::get_all_categories))
                            .route(web::post().to_async(router::category::get_categories)),
                    )
                    .service(
                        web::resource("/popular/{page}")
                            .route(web::get().to_async(router::category::get_popular)),
                    )
                    .service(
                        web::resource("/{category_id}/{page}")
                            .route(web::get().to_async(router::category::get_category)),
                    ),
            )
            .service(
                web::scope("/test")
                    .service(
                        web::resource("/lock").route(web::get().to_async(router::test::test_global_var)),
                    )
                    .service(
                        web::resource("/generate_admin/{username}/{password}/{email}").route(web::get().to_async(router::test::generate_admin)),
                    )
            )
            .service(
                web::scope("/upload")
                    .service(
                        web::resource("/").route(web::post().to_async(router::stream::upload_file))
                    )
            )
            .service(fs::Files::new("/public", "./public"))
    })
        .bind(format!("{}:{}", &server_ip, &server_port))?
        .start();

    sys.run()
}
