#[macro_use]
extern crate serde_derive;

use std::env;

use actix::prelude::Actor;
use actix_web::{
    http::header,
    middleware::Logger,
    web::{self, ServiceConfig},
    App, HttpServer,
};
use dotenv::dotenv;

mod handler;
mod model;
mod router;
mod util;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    // std::env::set_var("RUST_LOG", "actix_server=info,actix_web=trace");
    // env_logger::init();

    let postgres_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set in .env");
    let server_ip = env::var("SERVER_IP").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let server_port = env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_owned());
    let cors_origin = env::var("CORS_ORIGIN").unwrap_or_else(|_| "All".to_owned());
    let use_mail = env::var("USE_MAIL")
        .unwrap_or_else(|_| "true".to_owned())
        .parse::<bool>()
        .unwrap_or(true);
    let use_sms = env::var("USE_SMS")
        .unwrap_or_else(|_| "false".to_owned())
        .parse::<bool>()
        .unwrap_or(false);
    let use_rep = env::var("USE_REPORT")
        .unwrap_or_else(|_| "false".to_owned())
        .parse::<bool>()
        .unwrap_or(false);

    // create or clear database tables as well as redis cache
    let args = env::args().collect::<Vec<String>>();
    let is_init =
        crate::util::startup::init_table_cache(&args, postgres_url.as_str(), redis_url.as_str())
            .await;

    // build_cache function will also update Global vars.
    crate::util::startup::build_cache(&postgres_url, &redis_url, is_init)
        .await
        .expect("Failed to create Global Variables");

    /*
        Global vars and connections pools use lazy_static!() so they don't have to be passed to App::data
        They are safe to access through out the server.
    */

    // initialize connection pool for postgres and redis.
    crate::handler::db::pool().init().await;
    crate::handler::cache::pool_redis().init().await;

    /*
        init_message_services function will start MailerService, SMSTask and ErrReportTask
        according to use_xxx settings in .env.

        They are actors handle email, sms and error reports.
        The returned addrs are used to send message to actors.
        The address of ErrReportService is passed to other actors and is used for sending error
        messages which will eventually landed at MailerService and/or SMSService.
    */
    let rep_addr =
        crate::handler::messenger::init_message_services(use_mail, use_sms, use_rep).await;

    /*
        CacheService is an actix actor run in main thread and handle redis info update and failed redis insertion retry.
        The returned addrs is passed to AppData.
    */
    let cache_addr = crate::handler::cache_update::CacheService::new(rep_addr.clone()).start();

    /*
        init_psn_service function will start PSNService. It is an actor runs in main thread.
        The returned addr is used to send messages to PSNService.
        Request to PSN data will hit local cache and db with a delayed schedule request.
    */
    let psn_addr = crate::handler::psn::init_psn_service(rep_addr).await;

    HttpServer::new(move || {
        /*
            This HttpServer use a cache pass through flow for data.
            Anything can't be find in redis will hit postgres and trigger a redis update.
            A failed insertion to postgres will be ignored and returned as an error.
            A failed insertion to redis after a successful postgres insertion will be passed to RedisFailedTask and retry from there.
            Most data have an expire time in redis or can be removed manually.
            Only a small portion of data are stored permanently in redis
            (Mainly the reply_count and reply_timestamp of topics/categories/posts). The online status and last online time for user
            Removing them will result in some ordering issue.
        */

        let cors = actix_cors::Cors::new()
            .allowed_origin(&cors_origin)
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![
                header::AUTHORIZATION,
                header::ACCEPT,
                header::CONTENT_TYPE,
            ])
            .max_age(3600)
            .finish();

        App::new()
            .data(psn_addr.clone())
            .data(cache_addr.clone())
            // TalkService is an actor handle web socket connections and communication between client web socket actors.
            .data(crate::handler::talk::TalkService.start())
            .wrap(Logger::default())
            .wrap(cors)
            .configure(conf_admin)
            .configure(conf_auth)
            .configure(conf_psn)
            .configure(conf_test)
            .configure(conf_comm)
            .service(web::resource("/upload").route(web::post().to(router::stream::upload_file)))
            .service(web::resource("/talk").to(router::talk::talk))
            .service(actix_files::Files::new("/public", "./public"))
    })
    .bind(format!("{}:{}", &server_ip, &server_port))?
    .run()
    .await
}

fn conf_admin(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            .service(web::resource("/user").route(web::post().to(router::admin::update_user)))
            .service(web::resource("/post").route(web::post().to(router::admin::update_post)))
            .service(web::resource("/topic").route(web::post().to(router::admin::update_topic)))
            .service(
                web::scope("/category")
                    .service(
                        web::resource("/remove/{category_id}")
                            .route(web::get().to(router::admin::remove_category)),
                    )
                    .service(
                        web::resource("/update")
                            .route(web::post().to(router::admin::update_category)),
                    )
                    .service(web::resource("").route(web::post().to(router::admin::add_category))),
            ),
    );
}

fn conf_auth(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .service(web::resource("/register").route(web::post().to(router::auth::register)))
            .service(web::resource("/login").route(web::post().to(router::auth::login)))
            .service(
                web::resource("/activation/mail")
                    .route(web::post().to(router::auth::add_activation_mail)),
            )
            .service(
                web::resource("/activation/mail/{uuid}")
                    .route(web::get().to(router::auth::activate_by_mail)),
            ),
    );
}

fn conf_psn(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/psn")
            .service(
                web::resource("/auth").route(web::get().to(router::psn::query_handler_with_jwt)),
            )
            .service(web::resource("/community").route(web::get().to(router::psn::community)))
            .service(web::resource("").route(web::get().to(router::psn::query_handler))),
    );
}

fn conf_test(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/test")
            .service(web::resource("/raw").route(web::get().to(router::test::raw)))
            .service(web::resource("/raw_cache").route(web::get().to(router::test::raw_cache)))
            .service(web::resource("/topic").route(web::get().to(router::test::add_topic)))
            .service(web::resource("/post").route(web::get().to(router::test::add_post))),
    );
}

fn conf_comm(cfg: &mut ServiceConfig) {
    cfg.service(web::resource("/categories").route(web::get().to(router::category::query_handler)))
        .service(
            web::scope("/post")
                .service(web::resource("/update").route(web::post().to(router::post::update)))
                .service(web::resource("/{pid}").route(web::get().to(router::post::get)))
                .service(web::resource("").route(web::post().to(router::post::add))),
        )
        .service(
            web::scope("/topic")
                .service(web::resource("/update").route(web::post().to(router::topic::update)))
                .service(
                    web::resource("")
                        .route(web::get().to(router::topic::query_handler))
                        .route(web::post().to(router::topic::add)),
                ),
        )
        .service(
            web::scope("/user")
                .service(web::resource("/update").route(web::post().to(router::user::update)))
                .service(web::resource("/{id}").route(web::get().to(router::user::get))),
        );
}
