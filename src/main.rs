//#![allow(unused_imports)]
//extern crate actix;
//extern crate actix_web;
//extern crate serde;
//extern crate chrono;
//extern crate dotenv;
//extern crate futures;
//extern crate r2d2;
//extern crate jsonwebtoken;
//extern crate rand;
//extern crate regex;
//extern crate lettre;
//extern crate r2d2_redis;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;

use actix_web::{web, App, HttpServer, middleware::{Logger, cors::Cors}, http::header};
use actix::prelude::*;
use actix_files as fs;


use diesel::{r2d2::ConnectionManager, PgConnection};
use r2d2_redis::{redis, r2d2 as redis_r2d2, RedisConnectionManager};
use dotenv::dotenv;

mod model;
mod handler;
mod router;
mod util;
mod schema;

fn main() -> std::io::Result<()> {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = std::env::var("REDIS_URL").unwrap_or("redis://127.0.0.1".to_string());
    let server_ip = std::env::var("SERVER_IP").unwrap_or("127.0.0.1".to_string());
    let server_port = std::env::var("SERVER_PORT").unwrap_or("8081".to_string());
    let cors_origin = std::env::var("CORS_ORIGIN").unwrap_or("*".to_string());

    // clear cache on start up for test purpose
//    let redis_client = r2d2_redis::redis::Client::open(redis_url.as_str()).unwrap();
//    let clear_cache = redis_client.get_connection().unwrap();
//    let _result: Result<usize, _> = redis::cmd("flushall").query(&clear_cache);

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let postgres_pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create postgres pool.");

    let cache_manager = RedisConnectionManager::new(redis_url.as_str()).unwrap();
    let redis_pool = redis_r2d2::Pool::builder()
        .build(cache_manager)
        .expect("Failed to crate redis pool.");

    HttpServer::new(move || {
        App::new()
            .data(postgres_pool.clone())
            .data(redis_pool.clone())
            .wrap(Logger::default())
            .wrap(Cors::new()
                      .allowed_origin(&cors_origin)
                      .allowed_methods(vec!["GET", "POST"])
                      .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                      .allowed_header(header::CONTENT_TYPE)
                      .max_age(3600),
            )
            .service(
                web::scope("/user")
                    .service(
                        web::resource("/")
                            .route(web::post().to_async(router::user::update_user))
                            .route(web::get().to_async(router::user::get_user)),
                    )
                    .service(
                        web::resource("/register").route(
                            web::post().to_async(router::user::register_user),
                        ),
                    )
                    .service(
                        web::resource("/login")
                            .route(web::post().to_async(router::user::login_user)
                            ),
                    ),
            )
            .service(
                web::scope("/post")
                    .service(
                        web::resource("/").route(
                            web::post().to_async(router::post::add_post)
                        )
                    )
                    .service(
                        web::resource("/{pid}").route(
                            web::get().to_async(router::post::get_post)
                        )
                    )
                    .service(
                        web::resource("/edit").route(
                            web::post().to_async(router::post::update_post)
                        )
                    )
            )
            .service(
                web::scope("/topic")
                    .service(
                        web::resource("/")
                            .route(web::post().to_async(router::topic::add_topic)
                            )
                    )
                    .service(
                        web::resource("/edit")
                            .route(web::post().to_async(router::topic::update_topic))
                    )
                    .service(
                        web::resource("/{topic_id}/{page}")
                            .route(web::get().to_async(router::topic::get_topic))
                    )
            )
            .service(
                web::scope("/categories")
                    .service(
                        web::resource("/")
                            .route(web::get().to_async(router::category::get_all_categories))
                            .route(web::post().to_async(router::category::get_categories))
                    )
                    .service(
                        web::resource("/popular/{page}")
                            .route(web::get().to_async(router::category::get_popular))

                    )
                    .service(
                        web::resource("/{category_id}/{page}")
                            .route(web::get().to_async(router::category::get_category))
                    )
            )
//            .service(
//                web::scope("/upload")
//                    .service(
//                        web::resource("/")
//                            .route(web::post().to_async(router::))
//                    )
//            )
            .service(fs::Files::new("/", "./public/"))
    })
        .bind(format!("{}:{}", &server_ip, &server_port))?
        .run()
}
