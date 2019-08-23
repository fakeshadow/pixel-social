use std::cell::RefCell;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use actix::prelude::{
    Actor, ActorContext, ActorFuture, Addr, Arbiter, AsyncContext, Context, ContextFutureSpawner, fut,
    Future,Running, WrapFuture,
};
use actix_web_actors::ws;
use futures::future::join_all;
// actor handle psn request
// psn service impl get queue from cache handler.
use psn_api_rs::PSN;
use redis::{aio::SharedConnection, Client as RedisClient};
use tokio_postgres::{Client, connect, Statement, tls::NoTls};

use crate::handler::talk::DisconnectRequest;
use crate::model::{
    common::{GlobalSessions, GlobalTalks},
    errors::ErrorReport,
    messenger::{Mailer, Twilio},
    post::Post,
    topic::Topic,
};

// websocket heartbeat and connection time out time.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub type TALK = Addr<TalkService>;
pub type MAILER = Addr<MessageService>;

// actor handles individual user's websocket connection and communicate with TalkService Actors.
pub struct WsChatSession {
    pub id: u32,
    pub hb: Instant,
    pub addr: TALK,
}

impl Actor for WsChatSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.addr.do_send(DisconnectRequest {
            session_id: self.id,
        });
        Running::Stop
    }
}

// actor the same as CacheService except it runs interval functions on start up.
pub struct CacheUpdateService {
    pub url: String,
    pub cache: Option<RefCell<SharedConnection>>,
    pub failed_topic: Mutex<Vec<Topic>>,
    pub failed_post: Mutex<Vec<Post>>,
}

impl Actor for CacheUpdateService {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.start_interval(ctx);
    }
}

impl CacheUpdateService {
    pub fn connect(redis_url: &str) -> Addr<CacheUpdateService> {
        let client =
            RedisClient::open(redis_url).unwrap_or_else(|_| panic!("Can't connect to cache"));
        let url = redis_url.to_owned();

        CacheUpdateService::create(move |ctx| {
            let addr = CacheUpdateService {
                url,
                cache: None,
                failed_topic: Mutex::new(vec![]),
                failed_post: Mutex::new(vec![]),
            };

            client
                .get_shared_async_connection()
                .map_err(|_| panic!("failed to get redis connection"))
                .into_actor(&addr)
                .and_then(|conn, addr, _| {
                    addr.cache = Some(RefCell::new(conn));
                    fut::ok(())
                })
                .wait(ctx);
            addr
        })
    }
}

// actor handles communication between websocket sessions actors
// with a database connection(each actor) for messages and talks query. a redis connection(each actor) for users' cache info query.
pub struct TalkService {
    pub talks: GlobalTalks,
    pub sessions: GlobalSessions,
    pub db: std::cell::RefCell<Client>,
    pub cache: SharedConnection,
    pub insert_pub_msg: Statement,
    pub insert_prv_msg: Statement,
    pub get_pub_msg: Statement,
    pub get_prv_msg: Statement,
    pub get_relations: Statement,
    pub join_talk: Statement,
}

impl Actor for TalkService {
    type Context = Context<Self>;
}

impl TalkService {
    pub fn init(
        postgres_url: &str,
        redis_url: &str,
        talks: GlobalTalks,
        sessions: GlobalSessions,
    ) -> impl Future<Item=Addr<TalkService>, Error=()> {
        let conn = connect(postgres_url, NoTls);

        RedisClient::open(redis_url)
            .unwrap_or_else(|_| panic!("Can't connect to cache"))
            .get_shared_async_connection()
            .map_err(|e| panic!("{:?}", e))
            .and_then(move |cache| {
                conn.map_err(|e| panic!("{:?}", e))
                    .and_then(move |(mut db, conn)| {
                        actix::spawn(conn.map_err(|e| panic!("{:?}", e)));
                        let p1 = db.prepare("INSERT INTO public_messages1 (talk_id, text, time) VALUES ($1, $2, $3)");
                        let p2 = db.prepare("INSERT INTO private_messages1 (from_id, to_id, text, time) VALUES ($1, $2, $3, $4)");
                        let p3 = db.prepare("SELECT * FROM public_messages1 WHERE talk_id = $1 AND time <= $2 ORDER BY time DESC LIMIT 999");
                        let p4 = db.prepare("SELECT * FROM private_messages1 WHERE to_id = $1 AND time <= $2 ORDER BY time DESC LIMIT 999");
                        let p5 = db.prepare("SELECT friends FROM relations WHERE id = $1");
                        let p6 = db.prepare("UPDATE talks SET users=array_append(users, $1) WHERE id= $2");

                        join_all(vec![p6, p5, p4, p3, p2, p1])
                            .map_err(|e| panic!("{:?}", e))
                            .and_then(move |mut vec| {
                                Ok(TalkService::create(move |_| {
                                    let insert_pub_msg = vec.pop().unwrap();
                                    let insert_prv_msg = vec.pop().unwrap();
                                    let get_pub_msg = vec.pop().unwrap();
                                    let get_prv_msg = vec.pop().unwrap();
                                    let get_relations = vec.pop().unwrap();
                                    let join_talk = vec.pop().unwrap();

                                    TalkService {
                                        talks,
                                        sessions,
                                        db: std::cell::RefCell::new(db),
                                        cache,
                                        insert_pub_msg,
                                        insert_prv_msg,
                                        get_pub_msg,
                                        get_prv_msg,
                                        get_relations,
                                        join_talk,
                                    }
                                }))
                            })
                    })
            })
    }
}

// actor handles error report, sending email and sms messages.
pub struct MessageService {
    pub url: String,
    pub cache: Option<RefCell<SharedConnection>>,
    pub mailer: Option<Mailer>,
    pub twilio: Option<Twilio>,
    pub error_report: ErrorReport,
}

impl Actor for MessageService {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.start_interval(ctx);
    }
}

impl MessageService {
    pub fn connect(redis_url: &str) -> MAILER {
        let client = RedisClient::open(redis_url).expect("failed to connect to redis server");

        let url = redis_url.to_owned();

        MessageService::create(move |ctx| {
            let addr = MessageService {
                url,
                cache: None,
                mailer: Self::generate_mailer(),
                twilio: Self::generate_twilio(),
                error_report: Self::generate_error_report(),
            };

            client
                .get_shared_async_connection()
                .map_err(|_| panic!("failed to get redis connection"))
                .into_actor(&addr)
                .and_then(|conn, addr, _| {
                    addr.cache = Some(RefCell::new(conn));
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
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                act.addr.do_send(DisconnectRequest { session_id: act.id });
                ctx.stop();
                return;
            }
            ctx.ping("");
        });
    }
}

pub struct PSNService {
    pub db_url: String,
    pub cache_url: String,
    pub is_active: bool,
    pub psn: PSN,
    pub db: Option<RefCell<Client>>,
    pub insert_trophy_title: Option<RefCell<Statement>>,
    pub cache: Option<RefCell<SharedConnection>>,
}

impl Actor for PSNService {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.start_interval(ctx);
    }
}

impl PSNService {
    pub fn connect(postgres_url: &str, redis_url: &str) -> Addr<PSNService> {
        let client = RedisClient::open(redis_url).expect("failed to connect to redis server");
        let conn = connect(postgres_url, NoTls);

        let db_url = postgres_url.to_owned();
        let cache_url = redis_url.to_owned();

        PSNService::create(move |ctx| {
            let act = PSNService {
                db_url,
                cache_url,
                is_active: false,
                psn: PSN::new(),
                db: None,
                insert_trophy_title: None,
                cache: None,
            };

            client
                .get_shared_async_connection()
                .map_err(|_| panic!("failed to get redis connection"))
                .into_actor(&act)
                .and_then(|cache, act, _| {
                    conn.map_err(|e| panic!("{:?}", e))
                        .into_actor(act)
                        .and_then(|(mut db, conn), act, ctx| {
                            Arbiter::spawn(conn.map_err(|e| panic!("{:?}", e)));

                            // a costly upsert statement as we want to mark user trophy title is_visible to false if for any reason user try to hide this title
                            // or even a progress reduce
                            let p1 = db.prepare(
                                "INSERT INTO psn_user_trophy_titles
                                (np_id, np_communication_id, progress, earned_platinum, earned_gold, earned_silver, earned_bronze, last_update_date)
                                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                                    ON CONFLICT (np_id, np_communication_id) DO UPDATE SET
                                        progress = CASE WHEN psn_user_trophy_titles.progress < EXCLUDED.progress
                                            THEN EXCLUDED.progress
                                            ELSE psn_user_trophy_titles.progress
                                            END,
                                        earned_platinum = CASE WHEN psn_user_trophy_titles.earned_platinum < EXCLUDED.earned_platinum
                                            THEN EXCLUDED.earned_platinum
                                            ELSE psn_user_trophy_titles.earned_platinum
                                            END,
                                        earned_gold = CASE WHEN psn_user_trophy_titles.earned_gold < EXCLUDED.earned_gold
                                            THEN EXCLUDED.earned_gold
                                            ELSE psn_user_trophy_titles.earned_gold
                                            END,
                                        earned_silver = CASE WHEN psn_user_trophy_titles.earned_silver < EXCLUDED.earned_silver
                                            THEN EXCLUDED.earned_silver
                                            ELSE psn_user_trophy_titles.earned_silver
                                            END,
                                        earned_bronze = CASE WHEN psn_user_trophy_titles.earned_bronze < EXCLUDED.earned_bronze
                                            THEN EXCLUDED.earned_bronze
                                            ELSE psn_user_trophy_titles.earned_bronze
                                            END,
                                        last_update_date = CASE WHEN psn_user_trophy_titles.last_update_date < EXCLUDED.last_update_date
                                            THEN EXCLUDED.last_update_date
                                            ELSE psn_user_trophy_titles.last_update_date
                                            END,
                                        is_visible = CASE WHEN psn_user_trophy_titles.progress > EXCLUDED.progress
                                            THEN FALSE
                                            ELSE TRUE
                                            END");

                            ctx.wait(
                                join_all(vec![p1])
                                    .map_err(|e| panic!("{}", e))
                                    .into_actor(act)
                                    .and_then(|mut vec: Vec<Statement>, act, _| {
                                        act.insert_trophy_title = vec.pop().map(RefCell::new);
                                        fut::ok(())
                                    })
                            );

                            act.db = Some(RefCell::new(db));
                            act.cache = Some(RefCell::new(cache));
                            fut::ok(())
                        })
                })
                .wait(ctx);
            act
        })
    }
}
