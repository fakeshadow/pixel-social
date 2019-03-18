#![allow(proc_macro_derive_resolution_fallback)]

extern crate actix;
extern crate actix_web;
extern crate serde;
extern crate chrono;
extern crate dotenv;
extern crate futures;
extern crate r2d2;
extern crate frank_jwt;

#[macro_use]
extern crate serde_json;
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
mod errors;
mod handler;
mod router;
mod util;
mod schema;

use model::db::DbExecutor;
use util::init_ids::init;

fn main() {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let sys = actix::System::new("PixelShare");

    // search database and find the largest uid,pid,tid and then populate the app state with them.
    let next_ids_vec = init(&database_url);
    let next_ids_struct = app::NextIds::create(next_ids_vec);

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let address :Addr<DbExecutor>  = SyncArbiter::start(4, move || DbExecutor(pool.clone()));

    server::new(move || app::create_app(address.clone(), next_ids_struct.clone()))
        .workers(4)
        .bind("127.0.0.1:3100")
        .expect("Can not bind to '127.0.0.1:3100'")
        .start();

    sys.run();
}