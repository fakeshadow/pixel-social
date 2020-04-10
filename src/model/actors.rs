use std::time::{Duration, Instant};

use actix::prelude::{Actor, ActorContext, Addr, AsyncContext, Running};
use actix_web_actors::ws;

use crate::handler::talk::DisconnectRequest;

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
            ctx.ping(b"");
        });
    }
}
