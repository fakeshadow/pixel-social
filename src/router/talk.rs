use std::time::Instant;

use actix::prelude::{
    ActorContext,
    AsyncContext,
    Handler,
    Message,
    StreamHandler,
};
use actix_web::{web::{Payload, Data}, Error, HttpResponse, HttpRequest};
use actix_web_actors::ws;
use serde::Deserialize;

use crate::util::jwt::JwtPayLoad;
use crate::model::{
    actors::{TALK, TalkService, WsChatSession},
    talk::SessionMessage,
};
use crate::handler::{
    talk::{
        Connect,
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

pub fn talk(
    req: HttpRequest,
    stream: Payload,
    talk: Data<TALK>,
) -> Result<HttpResponse, Error> {
    println!("connected");
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

impl Handler<SessionMessage> for WsChatSession {
    type Result = ();

    fn handle(&mut self, msg: SessionMessage, ctx: &mut Self::Context) { ctx.text(msg.0); }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for WsChatSession {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        match msg {
            ws::Message::Close(_) => ctx.stop(),
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => self.hb = Instant::now(),
            ws::Message::Text(t) => {
                let t = t.trim();
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
                if self.id <= 0 {
                    match v[0] {
                        "/auth" => auth(self, v[1], ctx),
                        _ => ctx.text("!!! Unauthorized command")
                    }
                } else {
                    match v[0] {
                        "/msg" => general_msg_handler::<ClientMessage>(self, v[1], ctx),
                        "/history" => general_msg_handler::<GetHistory>(self, v[1], ctx),
                        "/remove" => general_msg_handler::<RemoveUser>(self, v[1], ctx),
                        "/admin" => general_msg_handler::<Admin>(self, v[1], ctx),
                        // request talk_id 0 to get all talks details.
                        "/talks" => general_msg_handler::<GetTalks>(self, v[1], ctx),
                        // get users of one talk from talk_id
                        "/users" => general_msg_handler::<GetTalks>(self, v[1], ctx),
                        "/join" => general_msg_handler::<Join>(self, v[1], ctx),
                        "/create" => general_msg_handler::<Create>(self, v[1], ctx),
                        "/delete" => general_msg_handler::<Delete>(self, v[1], ctx),
                        _ => ctx.text("!!! Unknown command")
                    }
                }
            }
            _ => (),
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
    match JwtPayLoad::from(string) {
        Ok(j) => {
            session.id = j.user_id;
            let _ = session.addr
                .do_send(Connect {
                    session_id: session.id,
                    addr: ctx.address().recipient(),
                });
        }
        Err(_) => ctx.text("!!! Authentication failed")
    }
}