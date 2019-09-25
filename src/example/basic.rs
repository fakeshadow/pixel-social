/* An example to use async await with tokio-postgres with actix-web */

#[macro_use]
extern crate serde_derive;

use std::{
    cell::RefCell,
    convert::TryFrom,
    sync::{Arc, Mutex},
};

use actix::prelude::Future as Future01;
use actix_web::{App, Error, HttpResponse, HttpServer, web};
use futures::{compat::Future01CompatExt, future::{FutureExt, TryFutureExt}, TryStreamExt};
use tokio_postgres::{Client, NoTls, Row, Statement, types::Type};
use std::borrow::Borrow;

// tokio runtime needed or tokio-postgres won't connect.
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let workers = 12;
    let server_url = "127.0.0.1:3200";
    let db_url = "postgres://postgres:123@localhost/test";

    let mut sys = actix::System::new("actix-web async/await test");

    let dbs = Arc::new(Mutex::new(Vec::new()));
    // build data from async/await functions. use await directly at this point could freeze the runtime. convert them to future01 and run block_on to resolve the futures.
    for _i in 0..workers {
        let db = sys.block_on(convert_to_01(db_url)).unwrap();
        dbs.lock().unwrap().push(db);
    }

    HttpServer::new(move || {
        // Use clone if you want to share data between workers. This example use data for local worker only. So that each worker have a postgres connection
        let db = dbs.lock().unwrap().pop().unwrap();
        App::new()
            .data(db)
            .service(
                web::scope("/test")
                    .service(web::resource("/stdfuture").route(web::get().to_async(test_std)))
                    .service(web::resource("/future01").route(web::get().to_async(test_01)))
            )
    })
        .bind(server_url)
        .unwrap()
        .workers(workers)
        .start();
    sys.run()
}

// convert a future0.3 to future0.1 so it can be ran by actix_runtime
fn convert_to_01(url: &'static str) -> impl Future01<Item=DatabaseService, Error=ResError> {
    DatabaseService::init(url).boxed_local().compat()
}

// only exist to isolate async functions. You can use async-await macro crate for actix-web to remove this boilerplate
fn test_std(db: web::Data<DatabaseService>) -> impl Future01<Item=HttpResponse, Error=Error> {
    test_async_await(db).boxed_local().compat().map_err(|_| actix_web::error::ErrorInternalServerError("Mock Error"))
}
fn test_01() -> impl Future01<Item=HttpResponse, Error=Error> {
    test_async_await_from_future_01().boxed_local().compat().map_err(|_| actix_web::error::ErrorInternalServerError("Mock Error"))
}

// the statements and params are only mock.
async fn test_async_await(db: web::Data<DatabaseService>) -> Result<HttpResponse, ResError> {
    let ids = vec![1u32];

    let db = db.get_ref();

    let st = db.1.borrow();

    let t = db.0.borrow_mut()
        .query(&st, &[&ids])
        .error_into()
        .try_fold(Vec::new(), |mut v, row| {
            if let Ok(t) = Topic::try_from(row) {
                v.push(t);
            }
            futures::future::ok(v)
        }).await?;

    Ok(HttpResponse::Ok().json(&t))
}

// run future0.1 in async function.
async fn test_async_await_from_future_01() -> Result<HttpResponse, ResError> {
    let future01 = actix_web::client::Client::default()
        .get("http://www.rust-lang.org")
        .send();

    let result = future01.compat().await;

    println!("{:?}", result);

    Ok(HttpResponse::Ok().finish())
}


pub struct DatabaseService(pub RefCell<Client>, pub Statement);

impl DatabaseService {
    pub async fn init(postgres_url: &str) -> Result<DatabaseService, ResError> {
        let (mut c, connection) = tokio_postgres::connect(postgres_url, NoTls).await?;

        let connection = connection.map(|_| ());

        actix::spawn(connection.unit_error().boxed().compat());

        let st = c.prepare_typed("SELECT * FROM topics WHERE id=ANY($1)", &[Type::OID_ARRAY]).await?;

        /* use join future if you have multiple statement to take advantage of pipeline.

        let statements: Vec<Result<Statement, tokio_postgres::Error>> = futures::future::join_all(vec![.., <your prepares>]).await;

        */

        Ok(DatabaseService(RefCell::new(c), st))
    }
}

#[derive(Debug)]
pub enum ResError {
    Mock
}

impl From<tokio_postgres::Error> for ResError {
    fn from(_e: tokio_postgres::Error) -> ResError {
        ResError::Mock
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Topic {
    pub id: u32,
    pub user_id: u32,
    pub category_id: u32,
    pub title: String,
    pub body: String,
    pub thumbnail: String,
    pub is_locked: bool,
    pub is_visible: bool,
    pub reply_count: Option<u32>,
}

impl TryFrom<Row> for Topic {
    type Error = ResError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Topic {
            id: row.try_get(0)?,
            user_id: row.try_get(1)?,
            category_id: row.try_get(2)?,
            title: row.try_get(3)?,
            body: row.try_get(4)?,
            thumbnail: row.try_get(5)?,
            is_locked: row.try_get(8)?,
            is_visible: row.try_get(9)?,
            reply_count: None,
        })
    }
}