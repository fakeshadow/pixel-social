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
    errors::{RepError, ErrorReport},
    common::{GlobalTalks, GlobalSessions},
    messenger::{Mailer, Twilio},
};
use crate::handler::{
    talk::DisconnectRequest,
    messenger::ErrorReportMessage,
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

impl GetSharedConn for CacheUpdateService {}

impl GetSharedConn for MessageService {}


pub struct DatabaseServiceRaw {
    pub client: std::sync::Mutex<Client>,
    pub topics_by_id: Statement,
    pub posts_by_id: Statement,
    pub users_by_id: Statement,
    pub insert_topic: Statement,
    pub insert_post: Statement,
    pub insert_user: Statement,
}

impl DatabaseServiceRaw {
    pub fn init(postgres_url: &str) -> impl Future<Item=DatabaseServiceRaw, Error=()> {
        println!("constructed");
        let conn = connect(postgres_url, NoTls);

        conn.then(|r| match r {
            Err(e) => {
                panic!("{:?}", e);
            }
            Ok((mut c, conn)) => {
                actix_rt::spawn(conn.map_err(|e| panic!("{}", e)));

                let p1 = c.prepare("SELECT * FROM topics WHERE id = ANY($1)");
                let p2 = c.prepare("SELECT * FROM posts WHERE id = ANY($1)");
                let p3 = c.prepare("SELECT * FROM users WHERE id = ANY($1)");
                let p4 = c.prepare("INSERT INTO topics
                        (id, user_id, category_id, thumbnail, title, body, created_at, updated_at)
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                        RETURNING *");
                let p5 = c.prepare("INSERT INTO posts
                            (id, user_id, topic_id, category_id, post_id, post_content, created_at, updated_at)
                            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                            RETURNING *");
                let p6 = c.prepare("INSERT INTO users (id, username, email, hashed_password, avatar_url, signature)
                        VALUES ($1, $2, $3, $4, $5, $6)
                        RETURNING *");

                join_all(vec![p6, p5, p4, p3, p2, p1])
                    .map_err(move |e| {
                        panic!("{:?}", e);
                    })
                    .map(|mut v| DatabaseServiceRaw {
                        client: std::sync::Mutex::new(c),
                        topics_by_id: v.pop().unwrap(),
                        posts_by_id: v.pop().unwrap(),
                        users_by_id: v.pop().unwrap(),
                        insert_topic: v.pop().unwrap(),
                        insert_post: v.pop().unwrap(),
                        insert_user: v.pop().unwrap(),
                    })
            }
        })
    }
}


impl DatabaseService {
    pub fn connect(postgres_url: &str, rep: Option<ErrorReportRecipient>) -> DB {
        let conn = connect(postgres_url, NoTls);

        DatabaseService::create(move |ctx| {
            let act = DatabaseService {
                db: None,
                topics_by_id: None,
                posts_by_id: None,
                users_by_id: None,
                insert_topic: None,
                insert_post: None,
                insert_user: None,
            };

            conn.into_actor(&act)
                .then(move |r, act, ctx| match r {
                    Err(e) => {
                        send_rep(rep.as_ref(), RepError::Database);
                        panic!("{:?}", e);
                    }
                    Ok((mut db, conn)) => {
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
                            .map_err(move |e| {
                                send_rep(rep.as_ref(), RepError::Database);
                                panic!("{:?}", e);
                            })
                            .into_actor(act)
                            .and_then(|mut v, act, _| {
                                act.topics_by_id = v.pop();
                                act.posts_by_id = v.pop();
                                act.users_by_id = v.pop();
                                act.insert_post = v.pop();
                                act.insert_topic = v.pop();
                                act.insert_user = v.pop();

                                fut::ok(())
                            })
                        );
                        act.db = Some(db);
                        Arbiter::spawn(conn.map_err(|e| panic!("{:?}", e)));
                        fut::ok(())
                    }
                })
                .wait(ctx);
            act
        })
    }
}


impl CacheService {
    pub fn connect(redis_url: &str, rep: Option<ErrorReportRecipient>) -> CACHE {
        let c = RedisClient::open(redis_url)
            .unwrap_or_else(|e| {
                send_rep(rep.as_ref(), RepError::Redis);
                panic!("{:?}", e);
            });

        CacheService::create(move |ctx| {
            let addr = CacheService {
                cache: None,
            };

            c.get_shared_async_connection()
                .into_actor(&addr)
                .then(move |r, addr, _| match r {
                    Ok(conn) => {
                        addr.cache = Some(conn);
                        fut::ok(())
                    }
                    Err(e) => {
                        send_rep(rep.as_ref(), RepError::Redis);
                        panic!("{:?}", e);
                    }
                })
                .wait(ctx);
            addr
        })
    }
}


impl
CacheUpdateService {
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
        rep: Option<ErrorReportRecipient>,
    ) -> TALK {
        let conn = connect(postgres_url, NoTls);
        let cache = RedisClient::open(redis_url)
            .unwrap_or_else(|_| panic!("Can't connect to cache"));

        TalkService::create(move |ctx| {
            let act = TalkService {
                talks,
                sessions,
                db: None,
                cache: None,
                insert_pub_msg: None,
                insert_prv_msg: None,
                get_pub_msg: None,
                get_prv_msg: None,
                get_relations: None,
                join_talk: None,
            };

            cache.get_shared_async_connection()
                .map_err(|_| panic!("failed to get redis connection"))
                .into_actor(&act)
                .and_then(|conn, act, _| {
                    act.cache = Some(conn);
                    fut::ok(())
                })
                .wait(ctx);

            conn.into_actor(&act)
                .then(move |r, act, ctx| match r {
                    Err(e) => {
                        send_rep(rep.as_ref(), RepError::Database);
                        panic!("{:?}", e);
                    }
                    Ok((mut db, conn)) => {
                        let p1 = db.prepare("INSERT INTO public_messages1 (talk_id, text, time) VALUES ($1, $2, $3)");
                        let p2 = db.prepare("INSERT INTO private_messages1 (from_id, to_id, text, time) VALUES ($1, $2, $3, $4)");
                        let p3 = db.prepare("SELECT * FROM public_messages1 WHERE talk_id = $1 AND time <= $2 ORDER BY time DESC LIMIT 999");
                        let p4 = db.prepare("SELECT * FROM private_messages1 WHERE to_id = $1 AND time <= $2 ORDER BY time DESC LIMIT 999");
                        let p5 = db.prepare("SELECT friends FROM relations WHERE id = $1");
                        let p6 = db.prepare("UPDATE talks SET users=array_append(users, $1) WHERE id= $2");

                        ctx.wait(join_all(vec![p6, p5, p4, p3, p2, p1])
                            .map_err(move |e| {
                                send_rep(rep.as_ref(), RepError::Database);
                                panic!("{:?}", e);
                            })
                            .into_actor(act)
                            .and_then(|mut vec, act, _| {
                                act.insert_pub_msg = vec.pop();
                                act.insert_prv_msg = vec.pop();
                                act.get_pub_msg = vec.pop();
                                act.get_prv_msg = vec.pop();
                                act.get_relations = vec.pop();
                                act.join_talk = vec.pop();
                                fut::ok(())
                            }));

                        act.db = Some(db);
                        Arbiter::spawn(conn.map_err(|e| panic!("{:?}", e)));
                        fut::ok(())
                    }
                })
                .wait(ctx);
            act
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
            // ToDo: remove session from talk actor and make request to redis to update user
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                ctx.stop();
                return;
            }
            ctx.ping("");
        });
    }
}


fn send_rep(rep: Option<&ErrorReportRecipient>, e: RepError) {
    if let Some(a) = rep.as_ref() {
        let _ = a.do_send(ErrorReportMessage(e));
    }
}

