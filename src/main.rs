#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

use std::{
    env,
    sync::{Arc, Mutex},
};

use actix_web::{App, http::header, HttpServer, middleware::Logger, web};
use futures::future::{FutureExt, TryFutureExt};

use dotenv::dotenv;

use crate::util::startup::{build_cache, create_table, drop_table};

mod handler;
mod model;
mod router;
mod util;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    //    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=trace");
    //    env_logger::init();

    let postgres_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set in .env");
    let server_ip = env::var("SERVER_IP").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let server_port = env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_owned());
    let cors_origin = env::var("CORS_ORIGIN").unwrap_or_else(|_| "All".to_owned());

    let mut sys = actix::System::new("pixelshare_async_await");

    // create or clear database tables as well as redis cache
    let args: Vec<String> = env::args().collect();
    let mut is_init = false;
    for arg in args.iter() {
        if arg == "drop" {
            drop_table(&mut sys, &postgres_url);
            let _ = crate::handler::cache::clear_cache(&redis_url);
            std::process::exit(1);
        }
        if arg == "build" {
            let success = create_table(&mut sys, &postgres_url);
            if success {
                is_init = true;
            } else {
                println!("tables already exists. building cache with is_init = false");
            }
        }
    }

    // build_cache function returns global variables.
    let (global, global_talks, global_sessions) =
        build_cache(&mut sys, &postgres_url, &redis_url, is_init).unwrap();

    // only global is wrapped in web::Data, global_talks and global_sessions are passed to every TalkService actor.
    let global = web::Data::new(global);

    /*
        actix runtime only run on future0.1 so all async functions must be converted before running.
        so run async await directly in main function could result in a runtime freeze.
    */
    // CacheUpdateService is an async actor who pass it's recipient to CacheService
    let addr = sys
        .block_on(
            crate::handler::cache_update::CacheUpdateService::init(&redis_url)
                .boxed_local()
                .compat(),
        )
        .expect("Failed to create Cache Update Service");
    let recipient = addr.recipient();

    // MessageService is an async actor runs in main thread. Be ware a panic from this actor and cache update service will stop the whole server
    // (no unwrap is used in these actors and all errors are mapped to ())
    let _ = sys
        .block_on(
            crate::handler::messenger::MessageService::init(redis_url.as_str())
                .boxed_local()
                .compat(),
        )
        .expect("Failed to create Message Service");

    let _ = sys
        .block_on(
            crate::handler::psn::PSNService::init(postgres_url.as_str(), redis_url.as_str())
                .boxed_local()
                .compat(),
        )
        .expect("Failed to create Message Service");

    let dbs = Arc::new(Mutex::new(Vec::new()));
    let caches = Arc::new(Mutex::new(Vec::new()));
    let talks = Arc::new(Mutex::new(Vec::new()));

    // build data for individual work.
    let workers = 12;
    for _i in 0..workers {
        // db service and cache service are data struct contains postgres connection, prepared queries and redis connections.
        // They are not shared between workers.
        let db = sys
            .block_on(
                crate::handler::db::DatabaseService::init(postgres_url.as_str())
                    .boxed_local()
                    .compat(),
            )
            .unwrap();
        let cache = sys
            .block_on(
                crate::handler::cache::CacheService::init(redis_url.as_str(), recipient.clone())
                    .boxed_local()
                    .compat(),
            )
            .unwrap();
        // TalkService is an actor handle websocket connections and communication between client websocket actors.
        // Every worker have a talk service actor with a postgres connection and a redis connection.
        // global_talks and sessions are shared between all workers and talk service actors.
        let talk = sys
            .block_on(
                crate::handler::talk::TalkService::init(
                    postgres_url.as_str(),
                    redis_url.as_str(),
                    global_talks.clone(),
                    global_sessions.clone(),
                )
                    .boxed_local()
                    .compat(),
            )
            .unwrap();

        dbs.lock().unwrap().push(db);
        caches.lock().unwrap().push(cache);
        talks.lock().unwrap().push(talk);
    }

    HttpServer::new(move || {
        /*
            Use a cache pass through flow for data.
            Anything can't be find in redis will hit postgres and trigger an redis update.
            A failed insertion to postgres will be ignored and returned as an error.
            A failed insertion to redis after a successful postgres insertion will be passed to CacheUpdateService actor and retry from there.
            Most data have a expire time in redis or can be removed manually.
            Only a small portion of data are stored permanently in redis
            (Mainly the reply_count and reply_timestamp of topics/categories/posts). The online status and last online time for user
            Removing them will result in some ordering issue.
        */

        let db = dbs.lock().unwrap().pop().unwrap();
        let cache = caches.lock().unwrap().pop().unwrap();
        let talk = talks.lock().unwrap().pop().unwrap();

        App::new()
            .register_data(global.clone())
            .data(db)
            .data(cache)
            .data(talk)
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
            .service(web::scope("/categories").service(
                web::resource("").route(web::get().to_async(router::category::query_handler)),
            ))
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
                        web::resource("")
                            .route(web::get().to_async(router::topic::query_handler))
                            .route(web::post().to_async(router::topic::add)),
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
                web::scope("/psn")
                    .service(
                        web::resource("/auth")
                            .route(web::get().to_async(router::psn::query_handler_with_jwt)),
                    )
                    .service(
                        web::resource("/community")
                            .route(web::get().to_async(router::psn::community)),
                    )
                    .service(
                        web::resource("").route(web::get().to_async(router::psn::query_handler)),
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
        .bind(format!("{}:{}", &server_ip, &server_port))
        .unwrap()
        .workers(workers)
        .start();
    sys.run()
}
