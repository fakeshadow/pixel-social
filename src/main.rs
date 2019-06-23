#![allow(unused_imports)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

use std::env;

use actix::prelude::*;
use actix_web::{
    App,
    http::header,
    HttpServer, middleware::Logger, web,
};
use actix_cors::Cors;
use actix_files as fs;
use dotenv::dotenv;

mod handler;
mod model;
mod router;
mod util;

use crate::{
    handler::cache::clear_cache,
    model::actors::{
        CacheService, DatabaseService, TalkService, MailService,
    },
    util::startup::{
        create_table,
        drop_table,
        build_cache,
    },
};

fn main() -> std::io::Result<()> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = env::var("REDIS_URL").unwrap_or("redis://127.0.0.1".to_owned());
    let server_ip = env::var("SERVER_IP").unwrap_or("127.0.0.1".to_owned());
    let server_port = env::var("SERVER_PORT").unwrap_or("8080".to_owned());
    let cors_origin = env::var("CORS_ORIGIN").unwrap_or("All".to_owned());

    // create or clear database tables
    let args: Vec<String> = env::args().collect();

    if args.len() > 2 && &args[1] == "DROP_TABLES" && &args[2] == "true" {
        drop_table(&database_url);
        std::process::exit(1);
    }
    if args.len() > 2 && &args[1] == "BUILD_TABLES" && &args[2] == "true" {
        create_table(&database_url);
    };

    let _ = clear_cache(&redis_url);

    let global_arc = build_cache(&database_url, &redis_url).expect("Unable to build cache");

    let sys = System::new("PixelShare");

    let talk_service = TalkService::connect(&database_url, &redis_url);
    // mail service is not passed into data as we add mail queue into redis cache directly.
    let mail_service = MailService::connect(&redis_url);

    HttpServer::new(move || {
        let db = DatabaseService::connect(&database_url);
        let cache = CacheService::connect(&redis_url);
        App::new()
            .data(global_arc.clone())
            .data(talk_service.clone())
            .data(db)
            .data(cache)
            .wrap(Logger::default())
            .wrap(Cors::new()
                .allowed_origin(&cors_origin)
                .allowed_methods(vec!["GET", "POST"])
                .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT, header::CONTENT_TYPE])
                .max_age(3600))
            .service(web::scope("/admin")
                .service(web::resource("/user").route(web::post().to_async(router::admin::update_user)))
                .service(web::resource("/post").route(web::post().to_async(router::admin::update_post)))
                .service(web::resource("/topic").route(web::post().to_async(router::admin::update_topic)))
                .service(web::scope("/category")
//                    .service(web::resource("/delete/{category_id}").route(web::get().to_async(router::admin::remove_category)))
                    .service(web::resource("/update").route(web::post().to_async(router::admin::update_category)))
                    .service(web::resource("").route(web::post().to_async(router::admin::add_category)))
                )
            )
            .service(web::scope("/user")
                .service(web::resource("/register").route(web::post().to_async(router::user::register)))
                .service(web::resource("/login").route(web::post().to_async(router::user::login)))
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
                .service(web::resource("/{category_id}/{topic_id}/{page}").route(web::get().to_async(router::topic::get)))
                .service(web::resource("").route(web::post().to_async(router::topic::add)))
            )
            .service(web::scope("/categories")
                .service(web::resource("/popular/{page}").route(web::get().to_async(router::category::get_popular)))
                .service(web::resource("/{category_id}/{page}").route(web::get().to_async(router::category::get_category)))
                .service(web::resource("").route(web::get().to_async(router::category::get_all_categories)))
            )
            .service(web::scope("/test")
                .service(web::resource("/hello").route(web::get().to(router::test::hello_world)))
                .service(web::resource("/lock").route(web::get().to_async(router::test::test_global_var)))
            )
            .service(web::resource("/upload").route(web::post().to_async(router::stream::upload_file)))
            .service(web::resource("/talk").to_async(router::talk::talk))
            .service(fs::Files::new("/public", "./public"))
    }).bind(format!("{}:{}", &server_ip, &server_port))?.start();
    sys.run()
}
