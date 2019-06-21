use std::collections::HashMap;

use actix::prelude::*;
use tokio_postgres::{connect, Client, tls::NoTls, Statement};
use redis::Client as RedisClient;

use crate::model::{
    talk::{SessionMessage, Talk},
    errors::ServiceError,
};
use futures::future::IntoFuture;

pub type SharedConn = redis::aio::SharedConnection;
pub type Conn = redis::aio::Connection;
pub type DB = Addr<DatabaseService>;
pub type CACHE = Addr<CacheService>;
pub type TALK = Addr<TalkService>;
pub type MAIL = Addr<MailService>;

pub struct DatabaseService {
    pub db: Option<Client>,
    pub categories: Option<Statement>,
    pub posts_by_tid: Option<Statement>,
    pub users_by_id: Option<Statement>,
}

pub struct CacheService {
    pub cache: Option<SharedConn>
}

pub struct TalkService {
    pub sessions: HashMap<u32, Recipient<SessionMessage>>,
    pub talks: HashMap<u32, Talk>,
    pub db: Option<Client>,
    pub cache: Option<RedisClient>,
    pub get_talks: Option<Statement>,
}

pub struct MailService {
    pub cache: Option<SharedConn>
}

impl Actor for DatabaseService {
    type Context = Context<Self>;
}

impl Actor for CacheService {
    type Context = Context<Self>;
}

impl Actor for TalkService {
    type Context = Context<Self>;
}

impl Actor for MailService {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
    }
}

impl DatabaseService {
    pub fn connect(postgres_url: &str) -> DB {
        let hs = connect(postgres_url, NoTls);

        DatabaseService::create(move |ctx| {
            let addr = DatabaseService {
                db: None,
                categories: None,
                posts_by_tid: None,
                users_by_id: None,
            };

            hs.map_err(|_| panic!("Can't connect to database"))
                .into_actor(&addr)
                .and_then(|(mut db, conn), addr, ctx| {
                    let p1 = db.prepare("SELECT * FROM posts
                        WHERE topic_id=$1
                        ORDER BY id ASC
                        OFFSET $2
                        LIMIT 20");
                    let p2 = db.prepare("SELECT * FROM users WHERE id = ANY($1)");
                    let p3 = db.prepare("SELECT * FROM categories");

                    ctx.wait(p1
                        .join3(p2, p3)
                        .map_err(|_| panic!("query prepare failed"))
                        .into_actor(addr)
                        .and_then(|(st1, st2, st3), addr, _| {
                            addr.posts_by_tid = Some(st1);
                            addr.users_by_id = Some(st2);
                            addr.categories = Some(st3);
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

impl CacheService {
    pub fn connect(redis_url: &str) -> CACHE {
        let client = RedisClient::open(redis_url)
            .unwrap_or_else(|_| panic!("Can't connect to cache"));

        CacheService::create(move |ctx| {
            let addr = CacheService {
                cache: None
            };

            client.get_shared_async_connection()
                .map_err(|_| panic!("failed to get redis connection"))
                .into_actor(&addr)
                .and_then(|conn, addr, _| {
                    addr.cache = Some(conn);
                    fut::ok(())
                })
                .wait(ctx);
            addr
        })
    }
}

impl TalkService {
    pub fn connect(postgres_url: &str, redis_url: &str) -> TALK {
        let hs = connect(postgres_url, NoTls);
        let cache = RedisClient::open(redis_url)
            .unwrap_or_else(|_| panic!("Can't connect to cache"));

        TalkService::create(move |ctx| {
            let addr = TalkService {
                sessions: HashMap::new(),
                talks: HashMap::new(),
                db: None,
                cache: Some(cache),
                get_talks: None,
            };

            hs.map_err(|_| panic!("Can't connect to database"))
                .into_actor(&addr)
                .and_then(|(mut db, conn), addr, ctx| {
                    ctx.wait(db
                        .prepare("SELECT * FROM talks")
                        .map_err(|e| panic!("{}", e))
                        .into_actor(addr)
                        .and_then(|st, addr, _| {
                            addr.get_talks = Some(st);
                            fut::ok(())
                        }));

                    addr.db = Some(db);
                    Arbiter::spawn(conn.map_err(|e| panic!("{}", e)));
                    fut::ok(())
                })
                .wait(ctx);
            addr
        })
    }
}

impl MailService {
    pub fn connect(redis_url: &str) -> MAIL {
        let client = RedisClient::open(redis_url).expect("failed to connect to redis server");

        MailService::create(move |ctx| {
            let addr = MailService {
                cache: None
            };

            client.get_shared_async_connection()
                .map_err(|_| panic!("failed to get redis connection"))
                .into_actor(&addr)
                .and_then(|conn, addr, _| {
                    addr.cache = Some(conn);
                    fut::ok(())
                })
                .wait(ctx);
            addr
        })
    }
}