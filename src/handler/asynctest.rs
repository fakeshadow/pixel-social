use std::io;

use tokio_postgres::{connect, Client, TlsMode, Statement};
use futures::Future;

use actix::prelude::*;
use actix::fut;
use crate::model::category::Category;
use crate::model::errors::ServiceError;

pub struct PostgresConnection {
    db: Option<Client>,
    get_categories: Option<Statement>,
    get_topics: Option<Statement>,
    get_topic: Option<Statement>,
    get_users: Option<Statement>,
}

impl Actor for PostgresConnection {
    type Context = Context<Self>;
}

pub type DB = Addr<PostgresConnection>;

impl PostgresConnection {
    pub fn connect(postgres_url: &str) -> Addr<PostgresConnection> {
        let hs = connect(postgres_url.parse().unwrap(), TlsMode::None);

        PostgresConnection::create(move |ctx| {
            let addr = PostgresConnection {
                db: None,
                get_categories: None,
                get_topics: None,
                get_topic: None,
                get_users: None,
            };

            hs.map_err(|_| panic!("Can't connect to database"))
                .into_actor(&addr)
                .and_then(|(mut db, conn), addr, ctx| {
                    ctx.wait(
                        db.prepare("SELECT * FROM categories")
                            .map_err(|e| panic!("{}", e))
                            .into_actor(addr)
                            .and_then(|st, addr, _| {
                                addr.get_categories = Some(st);
                                fut::ok(())
                            })
                    );

                    addr.db = Some(db);
                    Arbiter::spawn(conn.map_err(|e| panic!("{}", e)));
                    fut::ok(())
                })
                .wait(ctx);
            addr
        })
    }
}

pub struct Test;

impl Message for Test {
    type Result = Result<Vec<Category>, ServiceError>;
}

impl Handler<Test> for PostgresConnection {
    type Result = ResponseFuture<Vec<Category>, ServiceError>;

    fn handle(&mut self, msg: Test, _: &mut Self::Context) -> Self::Result {
        let categories = Vec::new();
        Box::new(self.db
            .as_ref()
            .unwrap()
            .query(self.get_categories.as_ref().unwrap(), &[])
            .map_err(|_| ServiceError::BadRequest)
            .fold(categories, move |mut categories, row| {
                categories.push(Category {
                    id: row.get(0),
                    name: row.get(1),
                    topic_count: row.get(2),
                    post_count: row.get(3),
                    subscriber_count: row.get(4),
                    thumbnail: row.get(5),
                });
                Ok::<_, ServiceError>(categories)
            })
            .and_then(|r| Ok(r)))
//        Box::new(match self.db.clone() {
//            Some(c) => {
//                c.prepare("SELECT * FROM categories")
//                    .map_err(|_| ServiceError::BadRequest)
//                    .and_then(move |s| {
//                        c.query(&s, &[])
//                            .map_err(|_| ServiceError::BadRequest)
//                            .fold(categories, move |mut categories, row| {
//                                categories.push(Category {
//                                    id: row.get(0),
//                                    name: row.get(1),
//                                    topic_count: row.get(2),
//                                    post_count: row.get(3),
//                                    subscriber_count: row.get(4),
//                                    thumbnail: row.get(5),
//                                });
//                                Ok::<_, ServiceError>(categories)
//                            })
//                    })
//                    .and_then(|r| Ok(r))
//            }
//            None => panic!("test")
//        })
    }
}