use std::time::{Duration, Instant};

use futures::future::join_all;

use actix::prelude::{
    fut, Actor,Arbiter, ActorContext, ActorFuture, Addr, AsyncContext, Context, ContextFutureSpawner,
    Future, Running, WrapFuture,
};
use actix_web_actors::ws;

// actor handle psn request
// psn service impl get queue from cache handler.
use psn_api_rs::PSN;
use redis::{aio::SharedConnection, Client as RedisClient};
use tokio_postgres::{connect, tls::NoTls, Client, Statement};

use crate::handler::talk::DisconnectRequest;
use crate::model::{
    common::{GlobalSessions, GlobalTalks},
    errors::ErrorReport,
    messenger::{Mailer, Twilio},
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
    pub cache: Option<SharedConnection>,
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

        CacheUpdateService::create(move |ctx| {
            let addr = CacheUpdateService { cache: None };

            client
                .get_shared_async_connection()
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
    ) -> impl Future<Item = Addr<TalkService>, Error = ()> {
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
    pub cache: Option<SharedConnection>,
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

        MessageService::create(move |ctx| {
            let addr = MessageService {
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
    pub is_active: bool,
    pub psn: PSN,
    pub db: Option<std::cell::RefCell<Client>>,
    pub insert_trophy_title: Option<Statement>,
    pub cache: Option<SharedConnection>,
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

        PSNService::create(move |ctx| {
            let act = PSNService {
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

                            let p1 = db.prepare(
                                "INSERT INTO psn_user_trophy_titles
                                (np_id, np_communication_id, progress, earned_platinum, earned_gold, earned_silver, earned_bronze, last_update_date)
                                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                                    ON CONFLICT (np_id, np_communication_id) DO UPDATE SET
                                        progress=EXCLUDED.progress,
                                        earned_platinum=EXCLUDED.earned_platinum,
                                        earned_gold=EXCLUDED.earned_gold,
                                        earned_silver=EXCLUDED.earned_silver,
                                        earned_bronze=EXCLUDED.earned_bronze,
                                        last_update_date=EXCLUDED.last_update_date");

                            ctx.wait(
                                join_all(vec![p1])
                                    .map_err(|e| panic!("{}", e))
                                    .into_actor(act)
                                    .and_then(|mut vec: Vec<Statement>, act, _| {
                                        act.insert_trophy_title = vec.pop();
                                        fut::ok(())
                                    })
                            );

                            act.db = Some(std::cell::RefCell::new(db));
                            act.cache = Some(cache);
                            fut::ok(())
                        })
                })
                .wait(ctx);
            act
        })
    }
}
