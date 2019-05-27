use std::time::{Duration, Instant};
use futures::{Future, stream::Stream};

use actix::prelude::*;
use actix_web::{web::{Payload, Data}, error, Error, HttpResponse, HttpRequest};
use actix_multipart::Multipart;
use actix_web_actors::ws;

use crate::model::{
    common::{RedisPool, PostgresPool},
    talk,
};
use crate::handler::{auth::UserJwt, stream::save_file, cache::get_unique_users_cache};

pub fn upload_file(
    _: UserJwt,
    multipart: Multipart,
) -> impl Future<Item=HttpResponse, Error=Error> {
    // ToDo: need to add an upload limit counter for user;
    multipart
        .map_err(error::ErrorInternalServerError)
        .map(|field| save_file(field).into_stream())
        .flatten()
        .collect()
        .map(|result| HttpResponse::Ok().json(result))
}

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub fn talk(
    req: HttpRequest,
    stream: Payload,
    db: Data<PostgresPool>,
    cache: Data<RedisPool>,
    srv: Data<Addr<talk::ChatServer>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        WsChatSession {
            id: 0,
            hb: Instant::now(),
            room: "Main".to_owned(),
            name: "Lobby".to_owned(),
            addr: srv.get_ref().clone(),
            cache: cache.get_ref().clone(),
        },
        &req,
        stream,
    )
}

struct WsChatSession {
    id: usize,
    hb: Instant,
    room: String,
    name: String,
    addr: Addr<talk::ChatServer>,
    cache: RedisPool,
}

impl Actor for WsChatSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        let addr = ctx.address();
        self.addr
            .send(talk::Connect {
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, mut act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    _ => ctx.stop(),
                }
                fut::ok(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.addr.do_send(talk::Disconnect { id: self.id });
        Running::Stop
    }
}

impl Handler<talk::SelfMessage> for WsChatSession {
    type Result = ();

    fn handle(&mut self, msg: talk::SelfMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
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
        match v[0] {
            "/auth" => {
                if v.len() == 2 {
                    match UserJwt::from(v[1]) {
                        Ok(token) => {
                            session.id = token.user_id as usize;
                            session.name = "ToDo".to_owned();
                            // ToDo: add to authenticated session
                            ctx.text("To Do");
                        }
                        Err(e) => ctx.text(format!("!!! {}", e.to_string()))
                    }
                } else { ctx.stop(); }
            }
            "/join" => {
                if v.len() == 2 {
                    session.room = v[1].to_owned();
                    session.addr.do_send(talk::Join {
                        id: session.id,
                        name: session.room.clone(),
                    });

                    ctx.text("joined");
                } else {
                    ctx.text("!!! room name is required");
                }
            }
            "/name" => {
                if v.len() == 2 {
                    session.name = v[1].to_owned();
                } else {
                    ctx.text("!!! name is required");
                }
            }
            _ => ctx.text(format!("!!! unknown command: {:?}", t)),
        }
    } else {
        session.addr.do_send(talk::ClientMessage {
            id: session.id,
            msg: format!("{}: {}", session.name, t),
            room: session.room.clone(),
        })
    }
}

impl WsChatSession {
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                act.addr.do_send(talk::Disconnect { id: act.id });
                ctx.stop();
                return;
            }
            ctx.ping("");
        });
    }
}