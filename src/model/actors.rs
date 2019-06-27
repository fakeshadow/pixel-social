use std::collections::HashMap;

use actix::prelude::*;
use futures::{future::IntoFuture, stream::futures_ordered};
use redis::Client as RedisClient;
use tokio_postgres::{Client, connect, Statement, tls::NoTls};

use crate::handler::db::talk_from_msg;
use crate::model::{
    errors::ServiceError,
    talk::{SessionMessage, Talk},
};

pub type SharedConn = redis::aio::SharedConnection;
pub type DB = Addr<DatabaseService>;
pub type CACHE = Addr<CacheService>;
pub type TALK = Addr<TalkService>;
pub type MAIL = Addr<MailService>;

pub struct DatabaseService {
    pub db: Option<Client>,
    pub topics_latest: Option<Statement>,
    pub topics_popular: Option<Statement>,
    pub topics_popular_all: Option<Statement>,
    pub topic_by_id: Option<Statement>,
    pub posts_popular: Option<Statement>,
    pub posts_old: Option<Statement>,
    pub users_by_id: Option<Statement>,
    pub insert_topic: Option<Statement>,
    pub insert_post: Option<Statement>,
    pub insert_user: Option<Statement>,
}

pub struct CacheService {
    pub cache: Option<SharedConn>
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
                topics_latest: None,
                topics_popular: None,
                topics_popular_all: None,
                topic_by_id: None,
                posts_popular: None,
                posts_old: None,
                users_by_id: None,
                insert_topic: None,
                insert_post: None,
                insert_user: None,
            };

            hs.map_err(|_| panic!("Can't connect to database"))
                .into_actor(&addr)
                .and_then(|(mut db, conn), addr, ctx| {
                    let p1 = db.prepare("SELECT * FROM topics
                        WHERE category_id = ANY($1)
                        ORDER BY last_reply_time DESC
                        OFFSET $2
                        LIMIT 20");
                    let p2 = db.prepare("SELECT * FROM topics
                        WHERE last_reply_time > $2 AND category_id = ANY($1)
                        ORDER BY reply_count DESC
                        OFFSET $3
                        LIMIT 20");
                    let p3 = db.prepare("SELECT * FROM topics
                        WHERE last_reply_time > $1
                        ORDER BY reply_count DESC
                        OFFSET $2
                        LIMIT 20");
                    let p4 = db.prepare("SELECT * FROM topics WHERE id = $1");
                    let p5 = db.prepare("SELECT * FROM posts
                        WHERE topic_id = $1
                        ORDER BY reply_count DESC, id ASC
                        OFFSET $2
                        LIMIT 20");
                    let p6 = db.prepare("SELECT * FROM posts
                        WHERE topic_id = $1
                        ORDER BY id ASC
                        OFFSET $2
                        LIMIT 20");
                    let p7 = db.prepare("SELECT * FROM users WHERE id = ANY($1)");
                    let p8 = db.prepare("INSERT INTO posts
                            (id, user_id, topic_id, category_id, post_id, post_content, created_at, updated_at, last_reply_time)
                            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                            RETURNING *");
                    let p9 = db.prepare("INSERT INTO topics
                        (id, user_id, category_id, thumbnail, title, body, created_at, updated_at, last_reply_time)
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                        RETURNING *");
                    let p10 = db.prepare("INSERT INTO users (id, username, email, hashed_password, avatar_url, signature)
                        VALUES ($1, $2, $3, $4, $5, $6)
                        RETURNING *");

                    let f = futures_ordered(vec![p1, p2, p3, p4, p5, p6, p7, p8, p9, p10]).collect();
                    ctx.wait(f
                        .map_err(|_| panic!("query prepare failed"))
                        .into_actor(addr)
                        .and_then(|mut v, addr, _| {
                            addr.insert_user = v.pop();
                            addr.insert_topic = v.pop();
                            addr.insert_post = v.pop();
                            addr.users_by_id = v.pop();
                            addr.posts_old = v.pop();
                            addr.posts_popular = v.pop();
                            addr.topic_by_id = v.pop();
                            addr.topics_popular_all = v.pop();
                            addr.topics_popular = v.pop();
                            addr.topics_latest = v.pop();
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

pub struct TalkService {
    pub sessions: HashMap<u32, Recipient<SessionMessage>>,
    pub talks: HashMap<u32, Talk>,
    pub db: Option<Client>,
    pub cache: Option<SharedConn>,
    pub insert_pub_msg: Option<Statement>,
    pub get_pub_msg: Option<Statement>,
    pub join_talk: Option<Statement>,
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
                cache: None,
                insert_pub_msg: None,
                get_pub_msg: None,
                join_talk: None,
            };

            cache.get_shared_async_connection()
                .map_err(|_| panic!("failed to get redis connection"))
                .into_actor(&addr)
                .and_then(|conn, addr, _| {
                    addr.cache = Some(conn);
                    fut::ok(())
                })
                .wait(ctx);

            hs.map_err(|_| panic!("Can't connect to database"))
                .into_actor(&addr)
                .and_then(|(mut db, conn), addr, ctx| {
                    let p1 = db.prepare("INSERT INTO public_messages1 (talk_id, message) VALUES ($1, $2)");
                    let p2 = db.prepare("SELECT * FROM public_messages1 WHERE talk_id = $1 AND time < $2 ORDER BY time DESC LIMIT 20");
                    let p3 = db.prepare("UPDATE talks SET users=array_append(users, $1) WHERE id= $2");

                    ctx.wait(db
                        .simple_query("SELECT * FROM talks")
                        .map_err(|e| panic!("{}", e))
                        .into_actor(addr)
                        .fold((), |_, row, addr, _, | {
                            if let Some(t) = talk_from_msg(&Some(row)).ok() {
                                addr.talks.insert(t.id, t);
                            };
                            fut::ok(())
                        }));

                    ctx.wait(p1.join3(p2, p3)
                        .map_err(|e| panic!("{}", e))
                        .into_actor(addr)
                        .and_then(|(st1, st2, st3), addr, _| {
                            addr.insert_pub_msg = Some(st1);
                            addr.get_pub_msg = Some(st2);
                            addr.join_talk = Some(st3);
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