#![allow(proc_macro_derive_resolution_fallback)]

extern crate actix;
extern crate actix_web;
extern crate serde;
extern crate chrono;
extern crate dotenv;
extern crate futures;
extern crate r2d2;
extern crate uuid;

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;


use models::DbExecutor;
use actix::prelude::*;
use actix_web::server;
use diesel::{r2d2::ConnectionManager, PgConnection};
use dotenv::dotenv;
use std::env;

use std::sync::{Arc, Mutex};

mod app;
mod models;
mod errors;
mod handler;
mod router;
mod ulti;
mod schema;

use ulti::init_ids::init;

fn main() {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let sys = actix::System::new("PixelShare");

    // search database and find the largest uid,pid,tid and then populate the app state with them.
    let next_ids = init(&database_url);

    let next_uid = Arc::new(Mutex::new(next_ids[0]));
    let next_pid = Arc::new(Mutex::new(next_ids[1]));
    let next_tid = Arc::new(Mutex::new(next_ids[2]));

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let address :Addr<DbExecutor>  = SyncArbiter::start(4, move || DbExecutor(pool.clone()));

    server::new(move || app::create_app(address.clone(), next_uid.clone(), next_pid.clone(), next_tid.clone()))
        .workers(4)
        .bind("127.0.0.1:3100")
        .expect("Can not bind to '127.0.0.1:3100'")
        .start();

    sys.run();
}