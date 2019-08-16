//#![allow(unused_imports)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

use std::env;

use actix::prelude::System;
use actix_web::{App, http::header, HttpServer, middleware::Logger, web};

use dotenv::dotenv;

use crate::{
    model::actors::{CacheUpdateService, MessageService, PSNService},
    util::startup::{build_cache, create_table, drop_table},
};

mod handler;
mod model;
mod router;
mod util;

fn main() -> std::io::Result<()> {
    dotenv().ok();
    //    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=trace");
    //    env_logger::init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set in .env");
    let server_ip = env::var("SERVER_IP").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let server_port = env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_owned());
    let cors_origin = env::var("CORS_ORIGIN").unwrap_or_else(|_| "All".to_owned());

    // create or clear database tables as well as redis cache
    let args: Vec<String> = env::args().collect();
    let mut is_init = false;
    for arg in args.iter() {
        if arg == "drop" {
            drop_table(&database_url);
            let _ = crate::handler::cache::clear_cache(&redis_url);
            std::process::exit(1);
        }
        if arg == "build" {
            let success = create_table(&database_url);
            if success {
                is_init = true;
            } else {
                println!("tables already exists. building cache with is_init = false");
            }
        }
    }

    // build_cache function returns global variables.
    let (global, global_talks, global_sessions) =
        build_cache(&database_url, &redis_url, is_init).expect("Unable to build cache");
    let global = web::Data::new(global);

    let mut sys = System::new("PixelShare");

    // cache update and message actor are not passed into data.
    let _ = CacheUpdateService::connect(&redis_url);
    let _ = MessageService::connect(&redis_url);

    // an actor handle PSN network request.
    let _ = PSNService::connect(&redis_url);

    // async connection pool test. currently running much slower than actor pattern.
    let pool = crate::router::test::build_pool(&mut sys);

    HttpServer::new(move || {
        // Use a cache pass through flow for data.
        // Anything can't be find in redis will hit postgres and trigger an redis update.
        // Most data have a expire time in redis or can be removed manually.
        // Only a small portion of data are stored permanently in redis
        // (Mainly the reply_count and reply_timestamp of topics/categories/posts). The online status and last online time for user
        // Removing them will result in some ordering issue.

        let talks = global_talks.clone();
        let sessions = global_sessions.clone();

        let db_url = database_url.clone();
        let rd_url = redis_url.clone();

        let db_url_2 = database_url.clone();
        let rd_url_2 = redis_url.clone();

        App::new()
            .register_data(global.clone())
            // The server will generate one async actor for each worker. The num of workers is tied to cpu core count.
            // talks and sessions are shared between threads. postgres and redis connections are local thread only.
            .data_factory(move || {
                crate::model::actors::TalkService::init(
                    db_url.as_str(),
                    rd_url.as_str(),
                    talks.clone(),
                    sessions.clone(),
                )
            })
            // db service and cache service are data struct contains postgres connection, prepared querys and redis connections.
            // They are not shared between threads.
            .data_factory(move || crate::handler::db::DatabaseService::init(db_url_2.as_str()))
            .data_factory(move || crate::handler::cache::CacheService::init(rd_url_2.as_str()))
            .data(pool.clone())
            .wrap(Logger::default())
            .wrap(
                actix_cors::Cors::new()
                    .allowed_origin(&cors_origin)
                    .allowed_methods(vec!["GET", "POST"])
                    .allowed_headers(vec![
                        header::AUTHORIZATION,
                        header::ACCEPT,
                        header::CONTENT_TYPE,
                    ])
                    .max_age(3600),
            )
            .service(
                web::scope("/admin")
                    .service(
                        web::resource("/user")
                            .route(web::post().to_async(router::admin::update_user)),
                    )
                    .service(
                        web::resource("/post")
                            .route(web::post().to_async(router::admin::update_post)),
                    )
                    .service(
                        web::resource("/topic")
                            .route(web::post().to_async(router::admin::update_topic)),
                    )
                    .service(
                        web::scope("/category")
                            .service(
                                web::resource("/remove/{category_id}")
                                    .route(web::get().to_async(router::admin::remove_category)),
                            )
                            .service(
                                web::resource("/update")
                                    .route(web::post().to_async(router::admin::update_category)),
                            )
                            .service(
                                web::resource("")
                                    .route(web::post().to_async(router::admin::add_category)),
                            ),
                    ),
            )
            .service(
                web::scope("/user")
                    .service(
                        web::resource("/update").route(web::post().to_async(router::user::update)),
                    )
                    .service(web::resource("/{id}").route(web::get().to_async(router::user::get))),
            )
            .service(
                web::scope("/post")
                    .service(
                        web::resource("/update").route(web::post().to_async(router::post::update)),
                    )
                    .service(web::resource("/{pid}").route(web::get().to_async(router::post::get)))
                    .service(web::resource("").route(web::post().to_async(router::post::add))),
            )
            .service(
                web::scope("/topic")
                    .service(
                        web::resource("/update").route(web::post().to_async(router::topic::update)),
                    )
                    .service(
                        web::resource("/popular/{topic_id}/{page}")
                            .route(web::get().to_async(router::topic::get_popular)),
                    )
                    .service(
                        web::resource("/{topic_id}/{page}")
                            .route(web::get().to_async(router::topic::get_oldest)),
                    )
                    .service(web::resource("").route(web::post().to_async(router::topic::add))),
            )
            .service(
                web::scope("/categories")
                    .service(
                        web::resource("/popular/all/{page}")
                            .route(web::get().to_async(router::category::get_popular_all)),
                    )
                    .service(
                        web::resource("/popular/{category_id}/{page}")
                            .route(web::get().to_async(router::category::get_popular)),
                    )
                    .service(
                        web::resource("/{category_id}/{page}")
                            .route(web::get().to_async(router::category::get_latest)),
                    )
                    .service(
                        web::resource("").route(web::get().to_async(router::category::get_all)),
                    ),
            )
            .service(
                web::scope("/auth")
                    .service(
                        web::resource("/register")
                            .route(web::post().to_async(router::auth::register)),
                    )
                    .service(
                        web::resource("/login").route(web::post().to_async(router::auth::login)),
                    )
                    .service(
                        web::resource("/activation/mail")
                            .route(web::post().to_async(router::auth::add_activation_mail)),
                    )
                    .service(
                        web::resource("/activation/mail/{uuid}")
                            .route(web::get().to_async(router::auth::activate_by_mail)),
                    ),
            )
            .service(
                web::scope("/psn")
                    .service(
                        web::resource("/auth")
                            .route(web::get().to_async(router::psn::auth)),
                    )
                    .service(
                        web::resource("/register")
                            .route(web::get().to_async(router::psn::register)),
                    )
                    .service(
                        web::resource("/profile").route(web::get().to_async(router::psn::profile)),
                    )
                    .service(
                        web::resource("/trophy").route(web::get().to_async(router::psn::trophy)),
                    ),
            )
            .service(
                web::scope("/test")
                    .service(web::resource("/raw").route(web::get().to_async(router::test::raw)))
                    .service(
                        web::resource("/raw_cache")
                            .route(web::get().to_async(router::test::raw_cache)),
                    )
                    .service(
                        web::resource("/l337_pool").route(web::get().to_async(router::test::pool)),
                    )
                    .service(
                        web::resource("/hello").route(web::get().to(router::test::hello_world)),
                    )
                    .service(
                        web::resource("/topic").route(web::get().to_async(router::test::add_topic)),
                    )
                    .service(
                        web::resource("/post").route(web::get().to_async(router::test::add_post)),
                    ),
            )
            .service(
                web::resource("/upload").route(web::post().to_async(router::stream::upload_file)),
            )
            .service(web::resource("/talk").to_async(router::talk::talk))
            .service(actix_files::Files::new("/public", "./public"))
    })
        .bind(format!("{}:{}", &server_ip, &server_port))?
        .start();
    sys.run()
}
