use actix::prelude::*;
use tokio_postgres::{connect, Client, tls::NoTls, Statement, SimpleQueryMessage};
use redis::Client as RedisClient;

use crate::model::errors::ServiceError;

pub type Conn = redis::aio::Connection;
pub type SharedConn = redis::aio::SharedConnection;
pub type DB = Addr<PostgresConnection>;
pub type CACHE = Addr<RedisConnection>;

pub struct PostgresConnection {
    pub db: Option<Client>,
    pub categories: Option<Statement>,
    pub posts_by_tid: Option<Statement>,
    pub users_by_id: Option<Statement>,
    pub next_tid: Option<Statement>,
    pub next_pid: Option<Statement>,
    pub next_uid: Option<Statement>,
}

pub struct RedisConnection {
    pub cache: Option<RedisClient>
}

impl Actor for PostgresConnection {
    type Context = Context<Self>;
}

impl Actor for RedisConnection {
    type Context = Context<Self>;
}

impl RedisConnection {
    pub fn connect(redis_url: &str) -> CACHE {
        let client = RedisClient::open(redis_url)
            .unwrap_or_else(|_| panic!("Can't connect to cache"));

        RedisConnection::create(move |ctx| {
            RedisConnection {
                cache: Some(client)
            }
        })
    }
}

impl PostgresConnection {
    pub fn connect(postgres_url: &str) -> DB {
        let hs = connect(postgres_url, NoTls);

        PostgresConnection::create(move |ctx| {
            let addr = PostgresConnection {
                db: None,
                categories: None,
                posts_by_tid: None,
                users_by_id: None,
                next_uid: None,
                next_tid: None,
                next_pid: None,
            };

            hs.map_err(|_| panic!("Can't connect to database"))
                .into_actor(&addr)
                .and_then(|(mut db, conn), addr, ctx| {
                    ctx.wait(
                        db.prepare("SELECT * FROM posts
                        WHERE topic_id=$1
                        ORDER BY id ASC
                        OFFSET $2
                        LIMIT 20")
                            .map_err(|e| panic!("{}", e))
                            .into_actor(addr)
                            .and_then(|st, addr, _| {
                                addr.posts_by_tid = Some(st);
                                fut::ok(())
                            })
                    );
                    ctx.wait(
                        db.prepare("SELECT * FROM users
                         WHERE id = ANY($1)")
                            .map_err(|e| panic!("{}", e))
                            .into_actor(addr)
                            .and_then(|st, addr, _| {
                                addr.users_by_id = Some(st);
                                fut::ok(())
                            })
                    );
                    ctx.wait(
                        db.prepare("SELECT * FROM categories")
                            .map_err(|e| panic!("{}", e))
                            .into_actor(addr)
                            .and_then(|st, addr, _| {
                                addr.categories = Some(st);
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