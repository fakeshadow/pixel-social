use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web::{web::{Payload, Data}, Error, HttpResponse, HttpRequest};
use actix_web_actors::ws;

use crate::util::jwt::JwtPayLoad;
use crate::model::{
    actors::TALK,
    talk::{
        Disconnect,
        Delete,
        Remove,
        Admin,
        SessionMessage,
    },
};
use crate::handler::{
    auth::UserJwt,
    talk::{
        Connect,
        GetRoomMembers,
        Create,
        GetTalks,
        Join,
        ClientMessage,
        GetHistory,
    },
};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub fn talk(
    req: HttpRequest,
    stream: Payload,
    talk: Data<TALK>,
) -> Result<HttpResponse, Error> {
    ws::start(
        WsChatSession {
            id: 0,
            hb: Instant::now(),
            addr: talk.get_ref().clone(),
        },
        &req,
        stream,
    )
}

struct WsChatSession {
    id: u32,
    hb: Instant,
    addr: TALK,
}

impl WsChatSession {
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                act.addr.do_send(Disconnect { session_id: act.id });
                ctx.stop();
                return;
            }
            ctx.ping("");
        });
    }
}

impl Handler<SessionMessage> for WsChatSession {
    type Result = ();

    fn handle(&mut self, msg: SessionMessage, ctx: &mut Self::Context) { ctx.text(msg.0); }
}

impl Actor for WsChatSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.addr.do_send(Disconnect { session_id: self.id });
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

// pattern match ws command
fn text_handler(session: &mut WsChatSession, text: String, ctx: &mut ws::WebsocketContext<WsChatSession>) {
    let t = text.trim();
    if !t.starts_with('/') {
        ctx.text("!!! Unknown command");
        ctx.stop();
        return;
    }
    let v: Vec<&str> = t.splitn(2, ' ').collect();
    if v.len() != 2 {
        ctx.text("!!! Empty command");
        ctx.stop();
        return;
    }
    if v[0].len() > 10 || v[1].len() > 2560 {
        ctx.text("!!! Message out of range");
        ctx.stop();
        return;
    }
    if session.id <= 0 {
        match v[0] {
            "/auth" => auth(session, v[1], ctx),
            _ => ctx.text("!!! Unauthorized command")
        }
    } else {
        match v[0] {
            "/msg" => match serde_json::from_str::<ClientMessage>(v[1]) {
                Ok(mut msg) => {
                    msg.session_id = session.id;
                    session.addr.do_send(msg);
                }
                Err(_) => ctx.text("!!! Query parsing error")
            }
            "/history" => match serde_json::from_str::<GetHistory>(v[1]) {
                Ok(mut msg) => {
                    msg.session_id = session.id;
                    session.addr.do_send(msg)
                }
                Err(_) => ctx.text("!!! Query parsing error")
            }
            "/remove" => match serde_json::from_str::<Remove>(v[1]) {
                Ok(mut msg) => {
                    msg.session_id = session.id;
                    session.addr.do_send(msg)
                }
                Err(_) => ctx.text("!!! Query parsing error")
            }
            "/admin" => match serde_json::from_str::<Admin>(v[1]) {
                Ok(mut msg) => {
                    msg.session_id = session.id;
                    session.addr.do_send(msg)
                }
                Err(_) => ctx.text("!!! Query parsing error")
            }
            // request talk_id 0 to get all talks details.
            "/talks" => get_talks(session, v[1], ctx),
            // get users of one talk from talk_id
            "/users" => get_talk_users(session, v[1], ctx),
            "/join" => join_talk(session, v[1], ctx),
            "/create" => create_talk(session, v[1], ctx),
            "/delete" => {
                let talk_id = v[1].parse::<u32>().unwrap_or(0);
                session.addr.do_send(Delete {
                    session_id: session.id,
                    talk_id,
                });
            }
            _ => ctx.text("!!! Unknown command")
        }
    }
}

fn auth(
    session: &mut WsChatSession,
    string: &str,
    ctx: &mut ws::WebsocketContext<WsChatSession>,
) {
    match serde_json::from_str::<String>(string) {
        Ok(s) => match JwtPayLoad::from(&s) {
            Ok(j) => {
                session.id = j.user_id;
                let _ = session.addr
                    .do_send(Connect {
                        session_id: session.id,
                        addr: ctx.address().recipient(),
                    });
            }
            Err(_) => ctx.text("!!! Authentication failed")
        },
        Err(_) => ctx.text("!!! Query parsing error")
    }
}

fn get_talk_users(
    session: &mut WsChatSession,
    string: &str,
    ctx: &mut ws::WebsocketContext<WsChatSession>,
) {
    match string.parse::<u32>() {
        Ok(talk_id) => session.addr
            .do_send(GetRoomMembers {
                session_id: session.id,
                talk_id,
            }),
        Err(_) => ctx.text("!!! Query parsing error")
    }
}

fn get_talks(
    session: &mut WsChatSession,
    string: &str,
    ctx: &mut ws::WebsocketContext<WsChatSession>,
) {
    match serde_json::from_str::<u32>(string) {
        Ok(talk_id) => session.addr.do_send(GetTalks {
            session_id: session.id,
            talk_id,
        }),
        Err(_) => ctx.text("!!! Query parsing error")
    }
}

fn create_talk(
    session: &mut WsChatSession,
    string: &str,
    ctx: &mut ws::WebsocketContext<WsChatSession>,
) {
    match serde_json::from_str::<Create>(string) {
        Ok(mut msg) => {
            msg.owner = session.id;
            session.addr.do_send(msg);
        }
        Err(_) => ctx.text("!!! Query parsing error")
    }
}

fn join_talk(
    session: &mut WsChatSession,
    string: &str,
    ctx: &mut ws::WebsocketContext<WsChatSession>,
) {
    match string.parse::<u32>() {
        Ok(talk_id) => session.addr
            .do_send(Join {
                talk_id,
                session_id: session.id,
            }),
        Err(_) => ctx.text("!!! Query parsing error")
    }
}