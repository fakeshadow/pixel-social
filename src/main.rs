#[macro_use]
extern crate serde_derive;

use std::env;

use actix_web::{http::header, middleware::Logger, App, HttpServer};

use crate::handler::data::DataRc;

mod config;
mod handler;
mod model;
mod router;
mod util;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "actix_server=info,actix_web=trace");

    env_logger::init();

    let env = crate::util::env::Env::from_env();

    // create or clear database tables as well as redis cache
    let args = env::args().collect::<Vec<String>>();
    let is_init =
        crate::util::startup::init_table_cache(&args, env.postgres_url(), env.redis_url()).await;

    // build_cache function will also update Global vars.
    let (talks, sessions) =
        crate::util::startup::build_cache(env.postgres_url(), env.redis_url(), is_init)
            .await
            .expect("Failed to create Global Variables");

    /*
        Global vars use once_cell so they don't have to be passed to App::data
        They are safe to access through out the server.
    */

    // initialize connection pool for postgres and redis.
    let db_pool = crate::handler::db::MyPostgresPool::new(env.postgres_url()).await;
    let cache_pool = crate::handler::cache::MyRedisPool::new(env.redis_url()).await;

    /*
        init_message_services function will start MailerService, SMSService and ErrReportService
        according to use_xxx settings in .env.

        They are actors handle email, sms and error reports.
        The returned addrs are used to send message to actors.
        The address of ErrReportService is passed to other actors and is used for sending error
        messages which will eventually landed at MailerService and/or SMSService.
    */
    let rep_addr = crate::handler::messenger::init_message_services(&env, cache_pool.clone()).await;

    /*
        CacheService is an actor run in main thread and handle redis info update and failed redis insertion retry.
        The returned addrs is passed to AppData.
    */
    let cache_addr = crate::handler::cache_update::init_cache_service(
        db_pool.clone(),
        cache_pool.clone(),
        rep_addr.clone(),
    )
    .await;

    /*
        init_psn_service function will start PSNService. It is an actor runs in main thread.
        The return addr is used to send messages to PSNService.
        Request to PSN data will hit local cache and db with a delayed schedule request.
    */
    let psn_addr =
        crate::handler::psn::init_psn_service(db_pool.clone(), cache_pool.clone(), rep_addr).await;

    // server address
    let addr = env.addr();

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
            .allowed_origin(env.cors_origin())
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![
                header::AUTHORIZATION,
                header::ACCEPT,
                header::CONTENT_TYPE,
            ])
            .max_age(3600)
            .finish();

        // We have clone here so that it can be moved into data_factory
        let db_pool = db_pool.clone();
        let cache_pool = cache_pool.clone();
        let talks = talks.clone();
        let sessions = sessions.clone();

        App::new()
            // All app data are wrapped in Rc to save clone cost.
            .app_data(DataRc::new(db_pool.clone()))
            .app_data(DataRc::new(cache_pool.clone()))
            .app_data(DataRc::new(psn_addr.clone()))
            .app_data(DataRc::new(cache_addr.clone()))
            // TalkService is an actor handle web socket connections and communication between
            // client web socket actors.
            .data_factory(move || {
                crate::handler::talk::init_talk_service(
                    db_pool.clone(),
                    cache_pool.clone(),
                    talks.clone(),
                    sessions.clone(),
                )
            })
            // .wrap(Logger::default())
            .wrap(cors)
            .configure(config::conf_admin)
            .configure(config::conf_auth)
            .configure(config::conf_psn)
            .configure(config::conf_test)
            .configure(config::conf_comm)
            .service(router::stream::upload_file)
            .service(router::talk::talk)
            .service(actix_files::Files::new("/public", "./public"))
    })
    .bind(addr)?
    .run()
    .await
}
