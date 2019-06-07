use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web::{web::{Payload, Data}, Error, HttpResponse, HttpRequest};
use actix_web_actors::ws;

use crate::model::talk;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub fn talk(
    req: HttpRequest,
    stream: Payload,
    srv: Data<Addr<talk::ChatService>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        WsChatSession {
            id: 1,
            hb: Instant::now(),
            addr: srv.get_ref().clone(),
        },
        &req,
        stream,
    )
}

struct WsChatSession {
    id: u32,
    hb: Instant,
    addr: Addr<talk::ChatService>,
}

impl Actor for WsChatSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        let addr = ctx.address();
        self.addr
            .send(talk::Connect {
                session_id: self.id,
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, _, ctx| {
                match res {
                    Ok(_) => {}
                    _ => ctx.stop(),
                }
                fut::ok(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.addr.do_send(talk::Disconnect { session_id: self.id });
        Running::Stop
    }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for WsChatSession {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => self.hb = Instant::now(),
            ws::Message::Text(t) => text_handler(self, t, ctx),
            ws::Message::Close(_) => ctx.stop(),
            _ => (),
        }
    }
}

fn text_handler(session: &mut WsChatSession, text: String, ctx: &mut ws::WebsocketContext<WsChatSession>) {
    let t = text.trim();
    if t.starts_with('/') {
        let v: Vec<&str> = t.splitn(2, ' ').collect();
        if v.len() != 2 {
            ctx.text("!!! illegal command");
            ctx.stop();
        } else {
            match v[0] {
                "/public" => {
                    let msg: Result<talk::PublicMessage, _> = serde_json::from_str(v[1]);
                    match msg {
                        Err(_) => ctx.text("!!! parsing error"),
                        Ok(msg) => session.addr.do_send(msg)
                    }
                }
                "/private" => {
                    let msg: Result<talk::PrivateMessage, _> = serde_json::from_str(v[1]);
                    match msg {
                        Err(_) => ctx.text("!!! parsing error"),
                        Ok(msg) => session.addr.do_send(msg)
                    }
                }
                /// get users of one room
                "/users" => {
                    let talk_id = v[1].parse::<u32>().unwrap_or(0);
                    let result = session.addr.do_send(talk::GetRoomMembers {
                        session_id: session.id,
                        talk_id,
                    });
                }
                /// request talk_id 0 to get all talks.
                "/talks" => {
                    let talk_id = v[1].parse::<u32>().unwrap_or(0);
                    let _ = session.addr.do_send(talk::GetTalks {
                        session_id: session.id,
                        talk_id,
                    });
                }
                "/join" => {
                    let talk_id = v[1].parse::<u32>().unwrap_or(0);
                    session.id = 1;
                    session.addr.do_send(talk::Join {
                        talk_id,
                        session_id: session.id,
                    });
                }
                "/create" => {
                    session.addr.send(talk::Create {
                        name: "".to_string(),
                        description: "".to_string(),
                        owner: session.id,
                    }).into_actor(session)
                        .then(|res, _, ctx| {
                            match res {
                                Ok(talk) => ctx.text(talk),
                                _ => ctx.stop(),
                            }
                            fut::ok(())
                        })
                        .wait(ctx);
                }
                _ => ctx.text("!!! unknown command")
            }
        }
    }
}

impl WsChatSession {
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                act.addr.do_send(talk::Disconnect { session_id: act.id });
                ctx.stop();
                return;
            }
            ctx.ping("");
        });
    }
}

impl Handler<talk::SessionMessage> for WsChatSession {
    type Result = ();

    fn handle(&mut self, msg: talk::SessionMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}