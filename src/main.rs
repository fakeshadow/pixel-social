//#![allow(unused_imports)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

use std::env;

use actix::prelude::System;
use actix_web::{
    App,
    http::header,
    HttpServer, middleware::Logger, web,
};
use dotenv::dotenv;

mod handler;
mod model;
mod router;
mod util;

use crate::{
    model::actors::{
        CacheService,
        CacheUpdateService,
        DatabaseService,
        TalkService,
        MessageService,
    },
    util::startup::{
        create_table,
        drop_table,
        build_cache,
    },
};

fn main() -> std::io::Result<()> {
    dotenv().ok();
//    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=trace");
//    env_logger::init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set in .env");
    let server_ip = env::var("SERVER_IP").unwrap_or("127.0.0.1".to_owned());
    let server_port = env::var("SERVER_PORT").unwrap_or("8080".to_owned());
    let cors_origin = env::var("CORS_ORIGIN").unwrap_or("All".to_owned());
    let use_report = env::var("USE_ERROR_REPORT").unwrap_or("false".to_owned()).parse::<bool>().unwrap_or(false);

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

    let mut sys = System::new("PixelShare");

    // cache update actor is not passed into data.
    let _ = CacheUpdateService::connect(&redis_url);

    // msg actor pass a recpient of RepError type message to other actors and handle.
    let msg = MessageService::connect(&redis_url);

    // a Option<Recipent> is passed to every actor for sending errors to message actor.
    let recipient = if use_report { Some(msg.recipient()) } else { None };

    // async connection pool test. currently running much slower than actor pattern.
//    let pool = crate::router::test::build_pool(&mut sys);


    HttpServer::new(move || {
        // Use a cache pass through flow for data. Anything can't be find in redis will hit database and trigger an cache update.
        // Most cache have a expire time or can be removed manually.
        // Only a small portion of data are stored permanently in redis (Mainly the reply_count and reply_timestamp of topics/categories/posts).
        // Removing them will result in some ordering issue.

        // the server will generate one async actor for each worker. The num of workers is tied to cpu core count.
        let db = DatabaseService::connect(&database_url, recipient.clone());
        let cache = CacheService::connect(&redis_url, recipient.clone());
        let talk = TalkService::connect(
            &database_url,
            &redis_url,
            global_talks.clone(),
            global_sessions.clone(),
            recipient.clone());

        App::new()
            .data(global.clone())
            .data(talk)
            .data(db)
            .data(cache)
            .data_factory(|| {
                let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
                crate::model::actors::DatabaseServiceRaw::init(database_url.as_str())
            })
//            .data(pool.clone())
            .wrap(Logger::default())
            .wrap(actix_cors::Cors::new()
                .allowed_origin(&cors_origin)
                .allowed_methods(vec!["GET", "POST"])
                .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT, header::CONTENT_TYPE])
                .max_age(3600))
            .service(web::scope("/admin")
                .service(web::resource("/user").route(web::post().to_async(router::admin::update_user)))
                .service(web::resource("/post").route(web::post().to_async(router::admin::update_post)))
                .service(web::resource("/topic").route(web::post().to_async(router::admin::update_topic)))
                .service(web::scope("/category")
                    .service(web::resource("/remove/{category_id}").route(web::get().to_async(router::admin::remove_category)))
                    .service(web::resource("/update").route(web::post().to_async(router::admin::update_category)))
                    .service(web::resource("").route(web::post().to_async(router::admin::add_category)))
                )
            )
            .service(web::scope("/user")
                .service(web::resource("/update").route(web::post().to_async(router::user::update)))
                .service(web::resource("/{id}").route(web::get().to_async(router::user::get)))
            )
            .service(web::scope("/post")
                .service(web::resource("/update").route(web::post().to_async(router::post::update)))
                .service(web::resource("/{pid}").route(web::get().to_async(router::post::get)))
                .service(web::resource("").route(web::post().to_async(router::post::add)))
            )
            .service(web::scope("/topic")
                .service(web::resource("/update").route(web::post().to_async(router::topic::update)))
                .service(web::resource("/popular/{topic_id}/{page}").route(web::get().to_async(router::topic::get_popular)))
                .service(web::resource("/{topic_id}/{page}").route(web::get().to_async(router::topic::get_oldest)))
                .service(web::resource("").route(web::post().to_async(router::topic::add)))
            )
            .service(web::scope("/categories")
                .service(web::resource("/popular/all/{page}").route(web::get().to_async(router::category::get_popular_all)))
                .service(web::resource("/popular/{category_id}/{page}").route(web::get().to_async(router::category::get_popular)))
                .service(web::resource("/{category_id}/{page}").route(web::get().to_async(router::category::get_latest)))
                .service(web::resource("").route(web::get().to_async(router::category::get_all)))
            )
            .service(web::scope("/auth")
                .service(web::resource("/register").route(web::post().to_async(router::auth::register)))
                .service(web::resource("/login").route(web::post().to_async(router::auth::login)))
                .service(web::resource("/activation/mail").route(web::post().to_async(router::auth::add_activation_mail)))
                .service(web::resource("/activation/mail/{uuid}").route(web::get().to_async(router::auth::activate_by_mail)))
            )
            .service(web::scope("/test")
                .service(web::resource("/raw").route(web::get().to_async(router::test::raw)))
                .service(web::resource("/pg_actor").route(web::get().to_async(router::test::actor)))
                .service(web::resource("/l337_pool").route(web::get().to_async(router::test::pool)))
                .service(web::resource("/hello").route(web::get().to(router::test::hello_world)))
                .service(web::resource("/topic").route(web::get().to_async(router::test::add_topic)))
                .service(web::resource("/post").route(web::get().to_async(router::test::add_post)))
            )
            .service(web::resource("/upload").route(web::post().to_async(router::stream::upload_file)))
            .service(web::resource("/talk").to_async(router::talk::talk))
            .service(actix_files::Files::new("/public", "./public"))
    }).bind(format!("{}:{}", &server_ip, &server_port))?.start();
    sys.run()
}
