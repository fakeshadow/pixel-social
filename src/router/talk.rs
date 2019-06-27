use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web::{web::{Payload, Data}, Error, HttpResponse, HttpRequest};
use actix_web_actors::ws;
use serde::Deserialize;

use crate::util::jwt::JwtPayLoad;
use crate::model::{
    actors::{TALK, TalkService},
    talk::SessionMessage,
};
use crate::handler::{
    auth::UserJwt,
    talk::{
        Connect,
        Disconnect,
        GetTalkUsers,
        Create,
        Delete,
        GetTalks,
        Join,
        Admin,
        RemoveUser,
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
            "/msg" => general_msg_handler::<ClientMessage>(session, v[1], ctx),
            "/history" => general_msg_handler::<GetHistory>(session, v[1], ctx),
            "/remove" => general_msg_handler::<RemoveUser>(session, v[1], ctx),
            "/admin" => general_msg_handler::<Admin>(session, v[1], ctx),
            // request talk_id 0 to get all talks details.
            "/talks" => get_talks(session, v[1], ctx),
            // get users of one talk from talk_id
            "/users" => general_msg_handler::<GetTalks>(session, v[1], ctx),
            "/join" => general_msg_handler::<Join>(session, v[1], ctx),
            "/create" => general_msg_handler::<Create>(session, v[1], ctx),
            "/delete" => general_msg_handler::<Delete>(session, v[1], ctx),
            _ => ctx.text("!!! Unknown command")
        }
    }
}

trait AttachSessionId {
    fn attach_session_id(&mut self, id: u32);
}

impl AttachSessionId for Admin {
    fn attach_session_id(&mut self, id: u32) {
        self.session_id = id;
    }
}

impl AttachSessionId for RemoveUser {
    fn attach_session_id(&mut self, id: u32) {
        self.session_id = id;
    }
}

impl AttachSessionId for GetHistory {
    fn attach_session_id(&mut self, id: u32) {
        self.session_id = id;
    }
}

impl AttachSessionId for ClientMessage {
    fn attach_session_id(&mut self, id: u32) {
        self.session_id = id;
    }
}

impl AttachSessionId for Join {
    fn attach_session_id(&mut self, id: u32) {
        self.session_id = id;
    }
}

impl AttachSessionId for Delete {
    fn attach_session_id(&mut self, id: u32) {
        self.session_id = id;
    }
}

impl AttachSessionId for Create {
    fn attach_session_id(&mut self, id: u32) {
        self.owner = id;
        self.session_id = id;
    }
}

impl AttachSessionId for GetTalks {
    fn attach_session_id(&mut self, id: u32) {
        self.session_id = id;
    }
}

fn general_msg_handler<'a, T>(
    session: &mut WsChatSession,
    text: &'a str,
    ctx: &mut ws::WebsocketContext<WsChatSession>,
) where T: AttachSessionId + Message + std::marker::Send + Deserialize<'a> + 'static,
        <T as Message>::Result: std::marker::Send,
        TalkService: Handler<T> {
    let r: Result<T, _> = serde_json::from_str::<T>(text);
    match r {
        Ok(mut msg) => {
            msg.attach_session_id(session.id);
            session.addr.do_send(msg)
        }
        Err(_) => ctx.text("!!! Query parsing error")
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