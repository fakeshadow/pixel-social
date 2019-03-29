#![allow(proc_macro_derive_resolution_fallback)]
extern crate actix;
extern crate actix_web;
extern crate serde;
extern crate chrono;
extern crate dotenv;
extern crate futures;
extern crate r2d2;
extern crate jsonwebtoken;
extern crate rand;
extern crate regex;
extern crate lettre;

extern crate tokio;
extern crate redis;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;

use std::env;
use std::sync::{Arc};

use actix::prelude::*;
use actix_web::server;
use diesel::{r2d2::ConnectionManager, PgConnection};
use dotenv::dotenv;

mod app;
mod model;
mod handler;
mod router;
mod util;
mod schema;

use model::db::{DbExecutor, CacheExecutor};

fn main() {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
//    let redis_url = env::var("REDIS_URL").unwrap_or("redis://127.0.0.1".to_string());
    let server_ip = env::var("SERVER_IP").unwrap_or("127.0.0.1".to_string());
    let server_port = env::var("SERVER_PORT").unwrap_or("8081".to_string());
    let sys = actix::System::new("PixelShare");

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let redis_client = redis::Client::open("redis://127.0.0.1").expect("Redis server connect failed");
    let redis_arc = Arc::new(redis_client);

    let db_addr: Addr<DbExecutor> = SyncArbiter::start(4, move || DbExecutor(pool.clone()));
    let cache_addr: Addr<CacheExecutor> = SyncArbiter::start(1,  move|| CacheExecutor(redis_arc.clone()));

    server::new(move || app::create_app(db_addr.clone(), cache_addr.clone()))
        .workers(4)
        .bind(format!("{}:{}", &server_ip, &server_port))
        .expect("Can not bind to target IP/Port")
        .start();

    sys.run();
}