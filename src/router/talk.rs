use std::time::Instant;

use actix::prelude::{
    ActorContext, ActorFuture, AsyncContext, Handler, Message, StreamHandler, WrapFuture,
};
use actix_web::{
    web::{Data, Payload},
    Error, HttpRequest, HttpResponse,
};
use actix_web_actors::ws;
use serde::Deserialize;

use crate::handler::talk::{
    Admin, AuthRequest, CheckPostgresMessage, ConnectRequest, CreateTalkRequest, DeleteTalkRequest,
    GetHistory, JoinTalkRequest, RemoveUserRequest, TalkByIdRequest, TalkService,
    TextMessageRequest, UserRelationRequest, UsersByIdRequest, TALK,
};
use crate::model::talk::SendMessage;
use crate::model::{actors::WsChatSession, talk::SessionMessage};
use crate::util::jwt::JwtPayLoad;

lazy_static! {
    static ref PARSING_ERROR: String = SendMessage::Error("Query Parsing Error").stringify();
    static ref DB_ERROR: String = SendMessage::Error("DataBase Error").stringify();
    static ref COMMAND_ERROR: String = SendMessage::Error("Empty Command").stringify();
    static ref AUTH_ERROR: String = SendMessage::Error("Unauthorized Command").stringify();
    static ref MSG_RANGE_ERROR: String = SendMessage::Error("Message Out of Range").stringify();
}

pub fn talk(req: HttpRequest, stream: Payload, talk: Data<TALK>) -> Result<HttpResponse, Error> {
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

    fn handle(&mut self, msg: SessionMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
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
                    ctx.text(COMMAND_ERROR.as_str());
                    return;
                }
                if v[0].len() > 10 || v[1].len() > 2560 {
                    ctx.text(MSG_RANGE_ERROR.as_str());
                    return;
                }
                if self.id == 0 {
                    match v[0] {
                        "/auth" => auth(self, v[1], ctx),
                        _ => ctx.text(AUTH_ERROR.as_str()),
                    }
                } else {
                    match v[0] {
                        "/msg" => general_msg_handler::<TextMessageRequest>(self, v[1], ctx),
                        "/history" => general_msg_handler::<GetHistory>(self, v[1], ctx),
                        "/remove" => general_msg_handler::<RemoveUserRequest>(self, v[1], ctx),
                        "/admin" => {
                            let text = v[1].to_owned();
                            ctx.spawn(
                                self.addr
                                    .send(CheckPostgresMessage)
                                    .into_actor(self)
                                    .map_err(|_, _, _| ())
                                    .map(move |r, act, ctx| {
                                        if let Err(_e) = r {
                                            ctx.text(DB_ERROR.as_str());
                                        };
                                        general_msg_handler::<Admin>(act, text.as_str(), ctx)
                                    }),
                            );
                        }
                        "/users" => general_msg_handler::<UsersByIdRequest>(self, v[1], ctx),
                        // request talk_id 0 to get all talks details.
                        "/talks" => general_msg_handler::<TalkByIdRequest>(self, v[1], ctx),
                        "/relation" => general_msg_handler::<UserRelationRequest>(self, v[1], ctx),
                        "/join" => general_msg_handler::<JoinTalkRequest>(self, v[1], ctx),
                        "/create" => general_msg_handler::<CreateTalkRequest>(self, v[1], ctx),
                        "/delete" => general_msg_handler::<DeleteTalkRequest>(self, v[1], ctx),
                        _ => ctx.text(COMMAND_ERROR.as_str()),
                    }
                }
            }
            _ => (),
        }
    }
}

trait SessionId {
    fn attach_session_id(&mut self, id: u32);
}

impl SessionId for Admin {
    fn attach_session_id(&mut self, id: u32) {
        self.session_id = Some(id);
    }
}

impl SessionId for RemoveUserRequest {
    fn attach_session_id(&mut self, id: u32) {
        self.session_id = Some(id);
    }
}

impl SessionId for GetHistory {
    fn attach_session_id(&mut self, id: u32) {
        self.session_id = Some(id);
    }
}

impl SessionId for TextMessageRequest {
    fn attach_session_id(&mut self, id: u32) {
        self.session_id = Some(id);
    }
}

impl SessionId for JoinTalkRequest {
    fn attach_session_id(&mut self, id: u32) {
        self.session_id = Some(id);
    }
}

impl SessionId for DeleteTalkRequest {
    fn attach_session_id(&mut self, id: u32) {
        self.session_id = Some(id);
    }
}

impl SessionId for CreateTalkRequest {
    fn attach_session_id(&mut self, id: u32) {
        self.owner = id;
        self.session_id = Some(id);
    }
}

impl SessionId for TalkByIdRequest {
    fn attach_session_id(&mut self, id: u32) {
        self.session_id = Some(id);
    }
}

impl SessionId for UsersByIdRequest {
    fn attach_session_id(&mut self, id: u32) {
        self.session_id = Some(id);
    }
}

impl SessionId for UserRelationRequest {
    fn attach_session_id(&mut self, id: u32) {
        self.session_id = Some(id);
    }
}

fn general_msg_handler<'a, T>(
    session: &mut WsChatSession,
    text: &'a str,
    ctx: &mut ws::WebsocketContext<WsChatSession>,
) where
    T: SessionId + Message + std::marker::Send + Deserialize<'a> + 'static,
    <T as Message>::Result: std::marker::Send,
    TalkService: Handler<T>,
{
    let r: Result<T, _> = serde_json::from_str::<T>(text);
    match r {
        Ok(mut msg) => {
            msg.attach_session_id(session.id);
            session.addr.do_send(msg)
        }
        Err(_) => ctx.text(PARSING_ERROR.as_str()),
    }
}

fn auth(session: &mut WsChatSession, text: &str, ctx: &mut ws::WebsocketContext<WsChatSession>) {
    let r: Result<AuthRequest, _> = serde_json::from_str(text);
    match r {
        Ok(auth) => match JwtPayLoad::from(&auth.token) {
            Ok(j) => {
                session.id = j.user_id;
                session.addr.do_send(ConnectRequest {
                    session_id: session.id,
                    online_status: auth.online_status,
                    addr: ctx.address(),
                });
            }
            Err(_) => ctx.text(PARSING_ERROR.as_str()),
        },
        Err(_) => ctx.text(PARSING_ERROR.as_str()),
    }
}
