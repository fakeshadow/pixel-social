use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web::{web::{Payload, Data}, Error, HttpResponse, HttpRequest};
use actix_web_actors::ws;

use crate::model::{
    actors::TALK,
    talk::{
        Connect,
        Disconnect,
        Create,
        Join,
        Delete,
        GetHistory,
        GetTalks,
        GetRoomMembers,
        Remove,
        Admin,
        ClientMessage,
        SessionMessage,
    },
};
use crate::handler::auth::UserJwt;
use crate::model::talk::GetLastTalkId;

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

impl Actor for WsChatSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        let addr = ctx.address();
        self.addr
            .send(Connect {
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

/// pattern match ws command
fn text_handler(session: &mut WsChatSession, text: String, ctx: &mut ws::WebsocketContext<WsChatSession>) {
    let t = text.trim();
    if t.starts_with('/') {
        let v: Vec<&str> = t.splitn(2, ' ').collect();
        if v.len() != 2 {
            ctx.text("!!! illegal command");
            ctx.stop();
        } else {
            match v[0] {
                "/msg" => {
                    let msg: Result<ClientMessage, _> = serde_json::from_str(v[1]);
                    match msg {
                        Ok(mut msg) => {
                            msg.session_id = session.id;
                            session.addr.do_send(msg);
                        }
                        Err(_) => ctx.text("!!! parsing error")
                    }
                }
                "/history" => {
                    let msg: Result<GetHistory, _> = serde_json::from_str(v[1]);
                    match msg {
                        Ok(mut msg) => {
                            msg.session_id = session.id;
                            session.addr.do_send(msg)
                        }
                        Err(_) => ctx.text("!!! parsing error")
                    }
                }
                /// get users of one talk from talk_id
                "/users" => {
                    let talk_id = v[1].parse::<u32>().unwrap_or(0);
                    let _ = session.addr.do_send(GetRoomMembers {
                        session_id: session.id,
                        talk_id,
                    });
                }
                /// request talk_id 0 to get all talks details.
                "/talks" => {
                    let talk_id = v[1].parse::<u32>().unwrap_or(0);
                    let _ = session.addr.do_send(GetTalks {
                        session_id: session.id,
                        talk_id,
                    });
                }
                "/join" => {
                    let talk_id = v[1].parse::<u32>().unwrap_or(0);
                    session.id = 1;
                    session.addr.do_send(Join {
                        talk_id,
                        session_id: session.id,
                    });
                }
                "/remove" => {
                    let msg: Result<Remove, _> = serde_json::from_str(v[1]);
                    match msg {
                        Ok(mut msg) => {
                            msg.session_id = session.id;
                            session.addr.do_send(msg)
                        }
                        Err(_) => ctx.text("!!! parsing error")
                    }
                }
                "/admin" => {
                    let msg: Result<Admin, _> = serde_json::from_str(v[1]);
                    match msg {
                        Ok(mut msg) => {
                            msg.session_id = session.id;
                            session.addr.do_send(msg)
                        }
                        Err(_) => ctx.text("!!! parsing error")
                    }
                }
                "/create" => {
                    let msg: Result<Create, _> = serde_json::from_str(v[1]);
                    match msg {
                        Ok(mut msg) => {
                            msg.owner = session.id;
                            session.addr
                                .send(GetLastTalkId)
                                .into_actor(session)
                                .then(|r, session, ctx| {
                                    match r {
                                        Ok(r) => match r {
                                            Ok(id) => {
                                                msg.talk_id = Some(id);
                                                session.addr.do_send(msg);
                                            }
                                            Err(_) => ctx.text("!!! failed to get new talk id")
                                        },
                                        Err(_) => ctx.text("!!! actor error")
                                    }
                                    fut::ok(())
                                })
                                .wait(ctx)
                        }
                        Err(_) => ctx.text("!!! parsing error")
                    }
                }
                "/delete" => {
                    let talk_id = v[1].parse::<u32>().unwrap_or(0);
                    session.addr.do_send(Delete {
                        session_id: session.id,
                        talk_id,
                    });
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

    fn handle(&mut self, msg: SessionMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}