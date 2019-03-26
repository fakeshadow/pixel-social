#![allow(proc_macro_derive_resolution_fallback)]
extern crate rand;
extern crate actix;
extern crate actix_web;
extern crate serde;
extern crate chrono;
extern crate dotenv;
extern crate futures;
extern crate r2d2;
extern crate jsonwebtoken;

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;

use actix::prelude::*;
use actix_web::server;
use diesel::{r2d2::ConnectionManager, PgConnection};
use dotenv::dotenv;
use std::env;

mod app;
mod model;
mod handler;
mod router;
mod util;
mod schema;

use model::db::DbExecutor;

fn main() {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let server_ip = env::var("SERVER_IP").unwrap_or("127.0.0.1".to_string());
    let server_port = env::var("SERVER_PORT").unwrap_or("8081".to_string());

    let sys = actix::System::new("PixelShare");

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let address: Addr<DbExecutor> = SyncArbiter::start(4, move || DbExecutor(pool.clone()));

    server::new(move || app::create_app(address.clone()))
        .workers(4)
        .bind(format!("{}:{}", &server_ip, &server_port))
        .expect("Can not bind to target IP/Port")
        .start();

    sys.run();
}