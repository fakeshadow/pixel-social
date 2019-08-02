use std::time::{Duration, Instant};
use futures::{future::join_all};

use actix::prelude::{
    Actor,
    ActorContext,
    ActorFuture,
    AsyncContext,
    Addr,
    Arbiter,
    Context,
    ContextFutureSpawner,
    fut,
    Future,
    Running,
    WrapFuture,
};
use actix_web_actors::ws;
use redis::Client as RedisClient;
use tokio_postgres::{
    Client,
    connect,
    Statement,
    tls::NoTls,
};

use crate::model::{
    errors::ErrorReport,
    common::{GlobalTalks, GlobalSessions},
    messenger::{Mailer, Twilio},
};
use crate::handler::{
    talk::DisconnectRequest,
};

// websocket heartbeat and connection time out time.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub type SharedConn = redis::aio::SharedConnection;
pub type DB = Addr<DatabaseService>;
pub type CACHE = Addr<CacheService>;
pub type TALK = Addr<TalkService>;
pub type MAILER = Addr<MessageService>;
pub type ErrorReportRecipient = actix::prelude::Recipient<crate::handler::messenger::ErrorReportMessage>;

// actor handles database query for categories ,topics, posts, users.
pub struct DatabaseService {
    pub db: Option<Client>,
    pub error_report: Option<ErrorReportRecipient>,
    pub topics_by_id: Option<Statement>,
    pub posts_by_id: Option<Statement>,
    pub users_by_id: Option<Statement>,
    pub insert_topic: Option<Statement>,
    pub insert_post: Option<Statement>,
    pub insert_user: Option<Statement>,
}

// actor handles communication between websocket sessions actors
// with a database connection(each actor) for messages and talks query. a redis connection(each actor) for users' cache info query.
pub struct TalkService {
    pub talks: GlobalTalks,
    pub sessions: GlobalSessions,
    pub db: Option<Client>,
    pub cache: Option<SharedConn>,
    pub error_report: Option<ErrorReportRecipient>,
    pub insert_pub_msg: Option<Statement>,
    pub insert_prv_msg: Option<Statement>,
    pub get_pub_msg: Option<Statement>,
    pub get_prv_msg: Option<Statement>,
    pub get_relations: Option<Statement>,
    pub join_talk: Option<Statement>,
}

// actor handles redis cache for categories,topics,posts,users.
pub struct CacheService {
    pub cache: Option<SharedConn>,
    pub error_report: Option<ErrorReportRecipient>,
}

// actor the same as CacheService except it runs interval functions on start up.
pub struct CacheUpdateService {
    pub cache: Option<SharedConn>
}

// actor handles error report, sending email and sms messages.
pub struct MessageService {
    pub cache: Option<SharedConn>,
    pub mailer: Mailer,
    pub twilio: Option<Twilio>,
    pub error_report: ErrorReport,
}

// actor handles individual user's websocket connection and communicate with TalkService Actors.
pub struct WsChatSession {
    pub id: u32,
    pub hb: Instant,
    pub addr: TALK,
}

impl Actor for DatabaseService {
    type Context = Context<Self>;
}

impl Actor for CacheService {
    type Context = Context<Self>;
}

impl Actor for CacheUpdateService {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.start_interval(ctx);
    }
}

impl Actor for TalkService {
    type Context = Context<Self>;
}

impl Actor for MessageService {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.start_interval(ctx);
    }
}

impl Actor for WsChatSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.addr.do_send(DisconnectRequest { session_id: self.id });
        Running::Stop
    }
}

trait GetSharedConn {
    fn get_conn(c: redis::Client) -> Box<dyn Future<Item=SharedConn, Error=()>> {
        Box::new(c
            .get_shared_async_connection()
            .map_err(|_| panic!("failed to get redis connection")))
    }
}

impl GetSharedConn for CacheService {}

impl GetSharedConn for CacheUpdateService {}

impl GetSharedConn for MessageService {}


impl DatabaseService {
    pub fn connect(postgres_url: &str, error_reprot: Option<ErrorReportRecipient>) -> DB {
        let hs = connect(postgres_url, NoTls);

        DatabaseService::create(move |ctx| {
            let addr = DatabaseService {
                db: None,
                error_report: error_reprot,
                topics_by_id: None,
                posts_by_id: None,
                users_by_id: None,
                insert_topic: None,
                insert_post: None,
                insert_user: None,
            };

            hs.map_err(|_| panic!("Can't connect to database"))
                .into_actor(&addr)
                .and_then(|(mut db, conn), addr, ctx| {
                    let p1 = db.prepare("SELECT * FROM topics WHERE id = ANY($1)");
                    let p2 = db.prepare("SELECT * FROM posts WHERE id = ANY($1)");
                    let p3 = db.prepare("SELECT * FROM users WHERE id = ANY($1)");
                    let p4 = db.prepare("INSERT INTO posts
                            (id, user_id, topic_id, category_id, post_id, post_content, created_at, updated_at)
                            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                            RETURNING *");
                    let p5 = db.prepare("INSERT INTO topics
                        (id, user_id, category_id, thumbnail, title, body, created_at, updated_at)
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                        RETURNING *");
                    let p6 = db.prepare("INSERT INTO users (id, username, email, hashed_password, avatar_url, signature)
                        VALUES ($1, $2, $3, $4, $5, $6)
                        RETURNING *");

                    ctx.wait(join_all(vec![p6, p5, p4, p3, p2, p1])
                        .map_err(|_| panic!("query prepare failed"))
                        .into_actor(addr)
                        .and_then(|mut v, addr, _| {
                            addr.topics_by_id = v.pop();
                            addr.posts_by_id = v.pop();
                            addr.users_by_id = v.pop();
                            addr.insert_post = v.pop();
                            addr.insert_topic = v.pop();
                            addr.insert_user = v.pop();

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
    pub fn connect(redis_url: &str, error_report: Option<ErrorReportRecipient>) -> CACHE {
        let client = RedisClient::open(redis_url)
            .unwrap_or_else(|_| panic!("Can't connect to cache"));

        CacheService::create(move |ctx| {
            let addr = CacheService {
                cache: None,
                error_report,
            };

            Self::get_conn(client)
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


impl CacheUpdateService {
    pub fn connect(redis_url: &str) -> Addr<CacheUpdateService> {
        let client = RedisClient::open(redis_url)
            .unwrap_or_else(|_| panic!("Can't connect to cache"));

        CacheUpdateService::create(move |ctx| {
            let addr = CacheUpdateService {
                cache: None
            };

            Self::get_conn(client)
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
    pub fn connect(
        postgres_url: &str,
        redis_url: &str,
        talks: GlobalTalks,
        sessions: GlobalSessions,
        error_report: Option<ErrorReportRecipient>,
    ) -> TALK {
        let hs = connect(postgres_url, NoTls);
        let cache = RedisClient::open(redis_url)
            .unwrap_or_else(|_| panic!("Can't connect to cache"));

        TalkService::create(move |ctx| {
            let addr = TalkService {
                talks,
                sessions,
                db: None,
                cache: None,
                error_report,
                insert_pub_msg: None,
                insert_prv_msg: None,
                get_pub_msg: None,
                get_prv_msg: None,
                get_relations: None,
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
                    let p1 = db.prepare("INSERT INTO public_messages1 (talk_id, text, time) VALUES ($1, $2, $3)");
                    let p2 = db.prepare("INSERT INTO private_messages1 (from_id, to_id, text, time) VALUES ($1, $2, $3, $4)");
                    let p3 = db.prepare("SELECT * FROM public_messages1 WHERE talk_id = $1 AND time <= $2 ORDER BY time DESC LIMIT 999");
                    let p4 = db.prepare("SELECT * FROM private_messages1 WHERE to_id = $1 AND time <= $2 ORDER BY time DESC LIMIT 999");
                    let p5 = db.prepare("SELECT friends FROM relations WHERE id = $1");
                    let p6 = db.prepare("UPDATE talks SET users=array_append(users, $1) WHERE id= $2");

                    ctx.wait(join_all(vec![p6, p5, p4, p3, p2, p1])
                        .map_err(|e| panic!("{}", e))
                        .into_actor(addr)
                        .and_then(|mut vec, addr, _| {
                            addr.insert_pub_msg = vec.pop();
                            addr.insert_prv_msg = vec.pop();
                            addr.get_pub_msg = vec.pop();
                            addr.get_prv_msg = vec.pop();
                            addr.get_relations = vec.pop();
                            addr.join_talk = vec.pop();
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


impl MessageService {
    pub fn connect(redis_url: &str) -> MAILER {
        let client = RedisClient::open(redis_url).expect("failed to connect to redis server");

        MessageService::create(move |ctx| {
            let addr = MessageService {
                cache: None,
                mailer: Self::generate_mailer(),
                twilio: Self::generate_twilio(),
                error_report: Self::generate_error_report(),
            };

            Self::get_conn(client)
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


impl WsChatSession {
    pub fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // ToDo: remove session from talk actor and make request to redis to update user's online status and last online time.
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                ctx.stop();
                return;
            }
            ctx.ping("");
        });
    }
}