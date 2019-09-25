#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

use std::{env, sync::Arc};

use actix_web::{
    http::header,
    middleware::Logger,
    web::{self, ServiceConfig},
    App, HttpServer,
};
use futures::{FutureExt, TryFutureExt};
use parking_lot::Mutex;

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

    let mut sys = actix::System::new("pixelshare_async_await");

    // create or clear database tables as well as redis cache
    let args = env::args().collect::<Vec<String>>();
    let is_init = crate::util::startup::init_table_cache(
        &mut sys,
        &args,
        postgres_url.as_str(),
        redis_url.as_str(),
    );

    /*
        actix runtime only run on future0.1 so all async functions must be converted before running.
        so run async await directly in main function could result in a runtime freeze.
    */

    // build_cache function returns global variables.
    // global_talks and global_sessions are passed to every TalkService actor.
    let (global, global_talks, global_sessions) = sys
        .block_on(
            crate::util::startup::build_cache(&postgres_url, &redis_url, is_init)
                .boxed_local()
                .compat(),
        )
        .unwrap();

    // global is wrapped in web::Data
    let global = web::Data::new(global);

    // MessageService runs in tokio thread pool and handle email, sms and error reports.
    // pass use_xxx env arg to determine if we want to spawn according handlers.
    // the returned rep_addr is an unbounded channel sender to send messages to MessageService to handle error reports(through email and sms service)
    let rep_addr = sys
        .block_on(
            crate::handler::messenger::MessageService::init(
                redis_url.as_str(),
                use_mail,
                use_sms,
                use_rep,
            )
            .boxed_local()
            .compat(),
        )
        .expect("Failed to create Message Service");

    // we pass the rep_addr as Option<_> to other services.
    // as if we set USE_REPORT = false then we don't want to send message for error report.
    let rep_addr = if use_rep { Some(rep_addr) } else { None };

    // CacheUpdateService runs in tokio thread pool and handle cache info update and failed insertion cache retry.
    // the returned addr is an unbounded channel sender to send messages to CacheUpdateService
    let addr = sys
        .block_on(
            crate::handler::cache_update::CacheUpdateService::init(&redis_url, rep_addr.clone())
                .boxed_local()
                .compat(),
        )
        .expect("Failed to create Cache Update Service");

    // PSNService spawned two futures that run in main thread.(So don't make it panic or it will bring down the whole application).
    // the returned psn is an unbounded channel sender to send messages to PSNService
    // Request to PSN data will hit local cache and db with a delayed schedule request.
    let psn = sys
        .block_on(
            crate::handler::psn::PSNService::init(
                postgres_url.as_str(),
                redis_url.as_str(),
                rep_addr.clone(),
            )
            .boxed_local()
            .compat(),
        )
        .expect("Failed to create Test Service");
    // we wrap it in web::Data just like global.
    let psn = web::Data::new(psn);

    let dbs = Arc::new(Mutex::new(Vec::new()));
    let caches = Arc::new(Mutex::new(Vec::new()));
    let talks = Arc::new(Mutex::new(Vec::new()));

    // build data for individual work.
    let workers = 12;
    for i in 0..workers {
        // db service and cache service are data struct contains postgres connection, prepared queries and redis connections.
        // They are not shared between workers.
        let db = sys
            .block_on(
                crate::handler::db::DatabaseService::init(postgres_url.as_str())
                    .boxed_local()
                    .compat(),
            )
            .unwrap_or_else(|_| panic!("Failed to create Database Service for worker : {}", i));
        let cache = sys
            .block_on(
                crate::handler::cache::CacheService::init(redis_url.as_str(), addr.clone())
                    .boxed_local()
                    .compat(),
            )
            .unwrap_or_else(|_| panic!("Failed to create Cache Service for worker : {}", i));

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
            .unwrap_or_else(|_| panic!("Failed to create Talk Service for worker : {}", i));

        dbs.lock().push(db);
        caches.lock().push(cache);
        talks.lock().push(talk);
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

        // unlock mutex and use them as App.data
        let db = dbs.lock().pop().unwrap();
        let cache = caches.lock().pop().unwrap();
        let talk = talks.lock().pop().unwrap();

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
            // global and psn are both wrapped in Data<Mutex> so use register_data to avoid double Arc
            .register_data(global.clone())
            .register_data(psn.clone())
            .data(db)
            .data(cache)
            .data(talk)
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
    .workers(workers)
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
