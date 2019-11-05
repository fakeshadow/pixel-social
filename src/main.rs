#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

use std::env;

use actix::Actor;
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

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    //        std::env::set_var("RUST_LOG", "actix_server=info,actix_web=trace");
    //        env_logger::init();

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

    // initialize connection pool for postgres.
    crate::handler::db::POOL.init().await;
    // initialize connection pool for redis.
    crate::handler::cache::POOL_REDIS.init().await;

    /*
        init_message_services function will start MailerTask, SMSTask and ErrReportTask according to use_xxx settings in .env.
        They are futures run in tokio thread pool and handle email, sms and error reports.
        The returned addrs are unbounded channel senders to send messages to Tasks.
        (We don't send message to MailerTask and SMSTask so the address is ignored)
        The address of ErrReportTask is passed to other tasks and is used for pushing error messages which will eventually landed at MailerTask and/or SMSTask.
    */
    let (_addr_ignore_1, _addr_ignore2, rep_addr) =
        crate::handler::messenger::init_message_services(use_mail, use_sms, use_rep);

    /*
        init_cache_update_service function will start RedisListTask and RedisFailedTask.
        They are futures run in tokio thread pool and handle redis info update and failed redis insertion retry.
        The returned addrs are unbounded channel senders to send messages to Tasks.
        (We don't send message to RedisListTask so the address is ignored)
    */
    let (redis_failed_addr, _addr_ignore) =
        crate::handler::cache_update::init_cache_update_services(rep_addr.clone());

    /*
        init_psn_service function will start PSNTask.
        It is a future runs in tokio thread pool and handle PSNRequest.
        The returned addr is a unbounded channel sender to send messages to PSNTask.
        Request to PSN data will hit local cache and db with a delayed schedule request.
    */
    let psn_addr = crate::handler::psn::init_psn_service(rep_addr);

    let sys = actix::System::new("pixel_share");

    /*
        actix runtime only run on future0.1 so all async functions must be converted before running.
        so run async await directly from this point in main function could result in a runtime freeze.
    */

    HttpServer::new(move || {
        /*
            This HttpServer use a cache pass through flow for data.
            Anything can't be find in redis will hit postgres and trigger an redis update.
            A failed insertion to postgres will be ignored and returned as an error.
            A failed insertion to redis after a successful postgres insertion will be passed to RedisFailedTask and retry from there.
            Most data have a expire time in redis or can be removed manually.
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
            .max_age(3600);

        App::new()
            .data(psn_addr.clone())
            .data(redis_failed_addr.clone())
            // TalkService is an actor handle websocket connections and communication between client websocket actors.
            .data(crate::handler::talk::TalkService.start())
            .wrap(Logger::default())
            .wrap(cors)
            .configure(conf_admin)
            .configure(conf_auth)
            .configure(conf_psn)
            .configure(conf_test)
            .configure(conf_comm)
            .service(
                web::resource("/upload").route(web::post().to_async(router::stream::upload_file)),
            )
            .service(web::resource("/talk").to_async(router::talk::talk))
            .service(actix_files::Files::new("/public", "./public"))
    })
    .bind(format!("{}:{}", &server_ip, &server_port))
    .unwrap()
    .start();

    sys.run()
}

fn conf_admin(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            .service(web::resource("/user").route(web::post().to_async(router::admin::update_user)))
            .service(web::resource("/post").route(web::post().to_async(router::admin::update_post)))
            .service(
                web::resource("/topic").route(web::post().to_async(router::admin::update_topic)),
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
                        web::resource("").route(web::post().to_async(router::admin::add_category)),
                    ),
            ),
    );
}

fn conf_auth(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .service(web::resource("/register").route(web::post().to_async(router::auth::register)))
            .service(web::resource("/login").route(web::post().to_async(router::auth::login)))
            .service(
                web::resource("/activation/mail")
                    .route(web::post().to_async(router::auth::add_activation_mail)),
            )
            .service(
                web::resource("/activation/mail/{uuid}")
                    .route(web::get().to_async(router::auth::activate_by_mail)),
            ),
    );
}

fn conf_psn(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/psn")
            .service(
                web::resource("/auth")
                    .route(web::get().to_async(router::psn::query_handler_with_jwt)),
            )
            .service(web::resource("/community").route(web::get().to_async(router::psn::community)))
            .service(web::resource("").route(web::get().to_async(router::psn::query_handler))),
    );
}

fn conf_test(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/test")
            .service(web::resource("/raw").route(web::get().to_async(router::test::raw)))
            .service(
                web::resource("/raw_cache").route(web::get().to_async(router::test::raw_cache)),
            )
            .service(web::resource("/topic").route(web::get().to_async(router::test::add_topic)))
            .service(web::resource("/post").route(web::get().to_async(router::test::add_post))),
    );
}

fn conf_comm(cfg: &mut ServiceConfig) {
    cfg.service(
        web::resource("/categories").route(web::get().to_async(router::category::query_handler)),
    )
    .service(
        web::scope("/post")
            .service(web::resource("/update").route(web::post().to_async(router::post::update)))
            .service(web::resource("/{pid}").route(web::get().to_async(router::post::get)))
            .service(web::resource("").route(web::post().to_async(router::post::add))),
    )
    .service(
        web::scope("/topic")
            .service(web::resource("/update").route(web::post().to_async(router::topic::update)))
            .service(
                web::resource("")
                    .route(web::get().to_async(router::topic::query_handler))
                    .route(web::post().to_async(router::topic::add)),
            ),
    )
    .service(
        web::scope("/user")
            .service(web::resource("/update").route(web::post().to_async(router::user::update)))
            .service(web::resource("/{id}").route(web::get().to_async(router::user::get))),
    );
}
