use std::cell::RefCell;
use std::time::{Duration, Instant};

use actix::prelude::{
    ActorContext,
    Actor, ActorFuture, Addr, AsyncContext, Context, ContextFutureSpawner,
    fut, Running, WrapFuture,
};
use actix_web_actors::ws;
use futures::{
    TryFutureExt,
    TryStreamExt,
};
use futures01::Future as Future01;
use psn_api_rs::PSN;
use redis::{aio::SharedConnection, Client as RedisClient};
use tokio_postgres::{Client, Statement};

use crate::handler::talk::DisconnectRequest;
use crate::model::{
    errors::{ErrorReport},
    messenger::{Mailer, Twilio},
};

// websocket heartbeat and connection time out time.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

// actor handles individual user's websocket connection and communicate with TalkService Actors.
pub struct WsChatSession {
    pub id: u32,
    pub hb: Instant,
    pub addr: Addr<crate::handler::talk::TalkService>,
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

//impl PSNService {
//    pub fn connect(postgres_url: &str, redis_url: &str) -> Addr<PSNService> {
//        let client = RedisClient::open(redis_url).expect("failed to connect to redis server");
//        let conn = connect(postgres_url, NoTls);
//
//        let db_url = postgres_url.to_owned();
//        let cache_url = redis_url.to_owned();
//
//        PSNService::create(move |ctx| {
//            let act = PSNService {
//                db_url,
//                cache_url,
//                is_active: false,
//                psn: PSN::new(),
//                db: None,
//                insert_trophy_title: None,
//                cache: None,
//            };
//
//            client
//                .get_shared_async_connection()
//                .map_err(|_| panic!("failed to get redis connection"))
//                .into_actor(&act)
//                .and_then(|cache, act, _| {
//                    conn.map_err(|e| panic!("{:?}", e))
//                        .into_actor(act)
//                        .and_then(|(mut db, conn), act, ctx| {
//                            Arbiter::spawn(conn.map_err(|e| panic!("{:?}", e)));
//
//                            // a costly upsert statement as we want to mark user trophy title is_visible to false if for any reason user try to hide this title
//                            // or even a progress reduce
//                            let p1 = db.prepare(
//                                "INSERT INTO psn_user_trophy_titles
//                                (np_id, np_communication_id, progress, earned_platinum, earned_gold, earned_silver, earned_bronze, last_update_date)
//                                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
//                                    ON CONFLICT (np_id, np_communication_id) DO UPDATE SET
//                                        progress = CASE WHEN psn_user_trophy_titles.progress < EXCLUDED.progress
//                                            THEN EXCLUDED.progress
//                                            ELSE psn_user_trophy_titles.progress
//                                            END,
//                                        earned_platinum = CASE WHEN psn_user_trophy_titles.earned_platinum < EXCLUDED.earned_platinum
//                                            THEN EXCLUDED.earned_platinum
//                                            ELSE psn_user_trophy_titles.earned_platinum
//                                            END,
//                                        earned_gold = CASE WHEN psn_user_trophy_titles.earned_gold < EXCLUDED.earned_gold
//                                            THEN EXCLUDED.earned_gold
//                                            ELSE psn_user_trophy_titles.earned_gold
//                                            END,
//                                        earned_silver = CASE WHEN psn_user_trophy_titles.earned_silver < EXCLUDED.earned_silver
//                                            THEN EXCLUDED.earned_silver
//                                            ELSE psn_user_trophy_titles.earned_silver
//                                            END,
//                                        earned_bronze = CASE WHEN psn_user_trophy_titles.earned_bronze < EXCLUDED.earned_bronze
//                                            THEN EXCLUDED.earned_bronze
//                                            ELSE psn_user_trophy_titles.earned_bronze
//                                            END,
//                                        last_update_date = CASE WHEN psn_user_trophy_titles.last_update_date < EXCLUDED.last_update_date
//                                            THEN EXCLUDED.last_update_date
//                                            ELSE psn_user_trophy_titles.last_update_date
//                                            END,
//                                        is_visible = CASE WHEN psn_user_trophy_titles.progress > EXCLUDED.progress
//                                            THEN FALSE
//                                            ELSE TRUE
//                                            END");
//
//                            ctx.wait(
//                                join_all(vec![p1])
//                                    .map_err(|e| panic!("{}", e))
//                                    .into_actor(act)
//                                    .and_then(|mut vec: Vec<Statement>, act, _| {
//                                        act.insert_trophy_title = vec.pop().map(RefCell::new);
//                                        fut::ok(())
//                                    })
//                            );
//
//                            act.db = Some(RefCell::new(db));
//                            act.cache = Some(RefCell::new(cache));
//                            fut::ok(())
//                        })
//                })
//                .wait(ctx);
//            act
//        })
//    }
//}
