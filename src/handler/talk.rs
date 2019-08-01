use std::fmt::Write;

use actix::prelude::{
    Addr,
    ActorFuture,
    AsyncContext,
    Context,
    fut,
    Handler,
    Message,
    WrapFuture,
};
use chrono::{NaiveDateTime, Utc};
use hashbrown::HashMap;

use crate::model::{
    actors::{TalkService, WsChatSession},
    errors::ResError,
    talk::{Talk, PublicMessage, PrivateMessage, SessionMessage},
};
use crate::model::user::User;
use chrono::format::Numeric::Second;

impl TalkService {
    // ToDo: add online offline filter
    fn send_message_many(&mut self, id: u32, msg: &str) {
        if let Some(talk) = self.get_talk(&id) {
            for uid in talk.users.iter() {
                self.send_message(uid, msg);
            };
        };
    }

    fn send_message(&self, session_id: &u32, msg: &str) {
        if let Some(addr) = self.get_session(session_id) {
            let _ = addr.do_send(SessionMessage(msg.to_owned()));
        };
    }

    fn parse_send_res_error(&self, session_id: u32, e: &ResError) {
        if let Some(addr) = self.get_session(&session_id) {
            let _ = addr.do_send(SessionMessage(SendMessage::Error(e.stringify()).stringify()));
        }
    }

    fn get_talks(&self) -> Option<HashMap<u32, Talk>> {
        match self.talks.read() {
            Ok(t) => Some(t.clone()),
            Err(_) => None
        }
    }

    fn get_talk(&self, talk_id: &u32) -> Option<Talk> {
        match self.talks.read() {
            Ok(t) => t.get(talk_id).map(|t| t.clone()),
            Err(_) => None
        }
    }

    fn get_session(&self, session_id: &u32) -> Option<Addr<WsChatSession>> {
        match self.sessions.read() {
            Ok(s) => s
                .get(session_id)
                .map(|addr| addr.clone()),
            Err(_) => None
        }
    }

    fn insert_talk(&self, talk: Talk) {
        let _ = self.talks
            .write()
            .map_err(|_| ())
            .map(|mut t| {
                t.insert(talk.id, talk);
            });
    }

    fn remove_talk(&self, tid: &u32) -> Result<(), ResError> {
        self.talks
            .write()
            .map_err(|_| ResError::InternalServerError)
            .map(|mut t| {
                t.remove(tid);
            })
    }

    fn insert_user(&self, talk_id: u32, session_id: u32) {
        let _ = self.talks
            .write()
            .map_err(|_| ())
            .map(|mut t| {
                if let Some(t) = t.get_mut(&talk_id) {
                    t.users.push(session_id)
                };
            });
    }
}

#[derive(Deserialize)]
pub struct AuthRequest {
    pub token: String,
    pub online_status: u32,
}

#[derive(Message)]
pub struct ConnectRequest {
    pub session_id: u32,
    pub online_status: u32,
    pub addr: Addr<WsChatSession>,
}

#[derive(Deserialize, Message, Clone)]
pub struct CreateTalkRequest {
    pub session_id: Option<u32>,
    pub name: String,
    pub description: String,
    pub owner: u32,
}

#[derive(Deserialize, Message)]
pub struct DeleteTalkRequest {
    pub session_id: Option<u32>,
    pub talk_id: u32,
}

#[derive(Deserialize, Message)]
pub struct JoinTalkRequest {
    pub session_id: Option<u32>,
    pub talk_id: u32,
}

#[derive(Deserialize, Message)]
pub struct RemoveUserRequest {
    pub session_id: Option<u32>,
    user_id: u32,
    talk_id: u32,
}

#[derive(Message, Deserialize)]
pub struct UserRelationRequest {
    pub session_id: Option<u32>,
}

#[derive(Message, Deserialize)]
pub struct UsersByIdRequest {
    pub session_id: Option<u32>,
    user_id: Vec<u32>,
}

#[derive(Message, Deserialize)]
pub struct TalkByIdRequest {
    pub session_id: Option<u32>,
    pub talk_id: u32,
}

// pass Some(talk_id) in json for public message, pass None for private message
#[derive(Deserialize, Message)]
pub struct TextMessageRequest {
    pub text: String,
    pub talk_id: Option<u32>,
    pub user_id: Option<u32>,
    pub session_id: Option<u32>,
}

// pass talk id for talk public messages. pass none for private history message.
#[derive(Deserialize, Message)]
pub struct GetHistory {
    pub time: String,
    pub talk_id: Option<u32>,
    pub session_id: Option<u32>,
}

#[derive(Deserialize, Message)]
pub struct Admin {
    pub add: Option<u32>,
    pub remove: Option<u32>,
    pub talk_id: u32,
    pub session_id: Option<u32>,
}

#[derive(Serialize)]
#[serde(tag = "type", content = "content")]
enum SendMessage<'a> {
    PublicMessage(&'a Vec<PublicMessage>),
    PrivateMessage(&'a Vec<PrivateMessage>),
    Users(&'a Vec<User>),
    Talks(Vec<&'a Talk>),
    Friends(&'a Vec<u32>),
    Success(&'a str),
    Error(&'a str),
}

impl SendMessage<'_> {
    fn default_err() -> String {
        serde_json::to_string(&SendMessage::Error("Stringify error")).unwrap()
    }

    fn stringify(&self) -> String {
        serde_json::to_string(self).unwrap_or(Self::default_err())
    }
}

#[derive(Message)]
pub struct DisconnectRequest {
    pub session_id: u32,
}

impl Handler<DisconnectRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: DisconnectRequest, ctx: &mut Context<Self>) {
        let _ = self.sessions
            .write()
            .map_err(|_| self.send_message(&msg.session_id, SendMessage::Error("Disconnect Failed").stringify().as_str()))
            .map(|mut t| {
                ctx.spawn(self
                    .set_online_status(msg.session_id, 0, true)
                    .into_actor(self)
                    .map_err(|_, _, _| ())
                    .map(|_, _, _| ()));

                t.remove(&msg.session_id);
            });
    }
}

impl Handler<TextMessageRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: TextMessageRequest, ctx: &mut Context<Self>) {
// ToDo: batch insert messages to database.

        match msg.talk_id {
            Some(id) => {
                let now = Utc::now().naive_utc();
                ctx.spawn(self
                    .insert_pub_msg(&[&id, &msg.text, &now])
                    .into_actor(self)
                    .then(move |r, act, _| {
                        match r {
                            Ok(_) => {
                                let s = SendMessage::PublicMessage(&vec![PublicMessage {
                                    text: msg.text,
                                    time: now,
                                    talk_id: msg.talk_id.unwrap(),
                                }]).stringify();

                                act.send_message_many(id, s.as_str());
                            }
                            Err(e) => act.parse_send_res_error(msg.session_id.unwrap(), &e)
                        }
                        fut::ok(())
                    }));
            }
            None => {
                let id = match msg.user_id {
                    Some(id) => id,
                    None => return self.parse_send_res_error(msg.session_id.unwrap(), &ResError::NotFound)
                };
                let now = Utc::now().naive_utc();
                ctx.spawn(self
                    .insert_prv_msg(&[&msg.session_id.unwrap(), &id, &msg.text, &now])
                    .into_actor(self)
                    .then(move |r, act, _| match r {
                        Ok(_) => {
                            let s = SendMessage::PrivateMessage(&vec![
                                PrivateMessage {
                                    user_id: msg.user_id.unwrap(),
                                    text: msg.text,
                                    time: now,
                                }]).stringify();

                            act.send_message(&id, s.as_str());
                            fut::ok(())
                        }
                        Err(e) => {
                            act.parse_send_res_error(msg.session_id.unwrap(), &e);
                            fut::ok(())
                        }
                    }));
            }
        };
    }
}

impl Handler<ConnectRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: ConnectRequest, ctx: &mut Context<Self>) {
        let f = self
            .set_online_status(msg.session_id, msg.online_status, true)
            .into_actor(self)
            .then(|r, act, _| {
                match r {
                    Ok(_) => {
                        let _ = act.sessions
                            .write()
                            .map_err(|_| {
                                let _ = msg.addr.do_send(SessionMessage(SendMessage::Error("Connection Failed").stringify()));
                            })
                            .map(|mut t| {
                                let _ = msg.addr.do_send(SessionMessage(SendMessage::Success("Connection Success").stringify()));

                                t.insert(msg.session_id, msg.addr);
                            });
                    }
                    Err(e) => act.parse_send_res_error(msg.session_id, &e)
                };
                fut::ok(())
            });
        ctx.spawn(f);
    }
}

impl Handler<CreateTalkRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: CreateTalkRequest, ctx: &mut Context<Self>) {
        let query = "SELECT Max(id) FROM talks";

        let f = self
            .simple_query_single_row::<u32>(query, 0)
            .into_actor(self)
            // ToDo: handle error.
            .map_err(|_, _, _| ())
            .and_then(move |cid, act, _| {
                //ToDo: in case query array failed.
                let query = format!("
                    INSERT INTO talks
                    (id, name, description, owner, admin, users)
                    VALUES ({}, '{}', '{}', {}, ARRAY [{}], ARRAY [{}])
                    RETURNING *", cid, msg.name, msg.description, msg.owner, cid, cid);

                act.simple_query_one(query.as_str())
                    .into_actor(act)
                    .map_err(|_, _, _| ())
                    // ToDo: handle error.
                    .and_then(move |t, act, _| {
                        let s = serde_json::to_string(&t)
                            .unwrap_or("!!! Stringify Error. But Talk Creation is success".to_owned());
                        act.insert_talk(t);
                        act.send_message(&msg.owner, &s);
                        fut::ok(())
                    })
            });
        ctx.spawn(f);
    }
}

impl Handler<JoinTalkRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: JoinTalkRequest, ctx: &mut Context<Self>) {
        let session_id = msg.session_id.unwrap();
        let talk_id = msg.talk_id;

        if let Some(talk) = self.get_talk(&talk_id) {
            if talk.users.contains(&session_id) {
                self.send_message(&session_id, "Already joined");
                return;
            };

            ctx.spawn(self
                .join_talk(&[&session_id, &talk_id])
                .into_actor(self)
                .then(move |r, act, _| {
                    match r {
                        Ok(_) => {
                            act.insert_user(talk_id, session_id);
                            act.send_message(&session_id, "!! Joined");
                        }
                        Err(e) => act.parse_send_res_error(session_id, &e)
                    };
                    fut::ok(())
                }));
        }
    }
}

impl Handler<TalkByIdRequest> for TalkService {
    type Result = ();
    fn handle(&mut self, msg: TalkByIdRequest, _: &mut Context<Self>) {
        match self.get_talks() {
            Some(t) => {
                let t = match msg.talk_id {
                    0 => t.iter().map(|(_, t)| t).collect(),
                    _ => t.get(&msg.talk_id).map(|t| vec![t]).unwrap_or(vec![])
                };

                self.send_message(&msg.session_id.unwrap(), SendMessage::Talks(t).stringify().as_str());
            }
            None => self.parse_send_res_error(msg.session_id.unwrap(), &ResError::NotFound)
        }
    }
}

impl Handler<UsersByIdRequest> for TalkService {
    type Result = ();
    fn handle(&mut self, msg: UsersByIdRequest, ctx: &mut Context<Self>) {
        if let Some(addr) = self.get_session(&msg.session_id.unwrap()) {
            let f = self
                .get_users_cache_from_ids(msg.user_id)
                .into_actor(self)
                .then(move |r, _, _| {
                    match r {
                        Ok(u) => addr
                            .do_send(SessionMessage(SendMessage::Users(&u).stringify())),
                        Err(e) => addr
                            .do_send(SessionMessage(e.stringify().to_owned()))
                    };
                    fut::ok(())
                });
            ctx.spawn(f);
        }
    }
}

impl Handler<UserRelationRequest> for TalkService {
    type Result = ();
    fn handle(&mut self, msg: UserRelationRequest, ctx: &mut Context<Self>) {
        let f = self
            .get_relations(&[msg.session_id.as_ref().unwrap()])
            .into_actor(self)
            .then(move |r, act, _| {
                match r {
                    Ok(r) => act.send_message(
                        msg.session_id.as_ref().unwrap(),
                        SendMessage::Friends(&r.friends).stringify().as_str()),
                    Err(e) => act.parse_send_res_error(msg.session_id.unwrap(), &e)
                };
                fut::ok(())
            });
        ctx.spawn(f);
    }
}

impl Handler<GetHistory> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: GetHistory, ctx: &mut Context<Self>) {
        let session_id = msg.session_id.unwrap();

        if let Some(addr) = self.get_session(&session_id) {
            let time = NaiveDateTime::parse_from_str(&msg.time, "%Y-%m-%d %H:%M:%S%.f")
                .unwrap_or(Utc::now().naive_utc());

            match msg.talk_id {
                Some(tid) => {
                    let f = self
                        .get_pub_msg(&[&tid, &time])
                        .into_actor(self)
                        .then(move |r: Result<Vec<PublicMessage>, ResError>, _, _| {
                            match r {
                                Ok(v) => addr
                                    .do_send(SessionMessage(SendMessage::PublicMessage(&v).stringify())),
                                Err(e) => addr
                                    .do_send(SessionMessage(e.stringify().to_owned()))
                            }
                            fut::ok(())
                        });
                    ctx.spawn(f);
                }
                // ToDo: add private message table and prepare statement
                None => {
                    let f = fut::ok(());
                    ctx.spawn(f);
                }
            };
        }
    }
}

impl Handler<RemoveUserRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: RemoveUserRequest, ctx: &mut Context<Self>) {
        let id = msg.session_id.unwrap();
        let tid = msg.talk_id;
        let uid = msg.user_id;

        if let Some(addr) = self.get_session(&id) {
            if let Some(talk) = self.get_talk(&tid) {
                if !talk.users.contains(&uid) {
                    let _ = addr.do_send(SessionMessage("!!! User not found in talk".to_owned()));
                    return;
                }

                let other_is_admin = talk.admin.contains(&uid);
                let other_is_owner = talk.owner == uid;
                let self_is_admin = talk.admin.contains(&id);
                let self_is_owner = talk.owner == id;

                let query = if self_is_owner && other_is_admin {
                    format!("UPDATE talks SET admin=array_remove(admin, {}), users=array_remove(users, {})
                WHERE id={} AND owner={}", uid, uid, tid, id)
                } else if (self_is_admin || self_is_owner) && !other_is_admin && !other_is_owner {
                    format!("UPDATE talks SET users=array_remove(users, {})
                WHERE id={}", uid, tid)
                } else {
                    let _ = addr.do_send(SessionMessage("!!! Unauthorized".to_owned()));
                    return;
                };

                let f = self
                    .simple_query_one::<Talk>(query.as_str())
                    .into_actor(self)
                    .then(move |r, act, _| {
                        match r {
                            Ok(t) => {
                                let s = serde_json::to_string(&t)
                                    .unwrap_or("!!! Stringify Error.But user removal success".to_owned());

                                act.insert_talk(t);
                                act.send_message_many(tid, &s);
                            }
                            Err(e) => {
                                let _ = addr.do_send(SessionMessage(e.stringify().to_owned()));
                            }
                        }
                        fut::ok(())
                    });

                ctx.spawn(f);
            }
        }
    }
}

impl Handler<Admin> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Admin, ctx: &mut Context<Self>) {
        let tid = msg.talk_id;
        let id = msg.session_id.unwrap();

        let mut query = "UPDATE talks SET admin=".to_owned();

        if let Some(uid) = msg.add {
            let _ = write!(&mut query, "array_append(admin, {})", uid);
        }

        if let Some(uid) = msg.remove {
            let _ = write!(&mut query, "array_remove(admin, {})", uid);
        }

        if query.ends_with("=") {
            self.send_message(&id, "!!! Empty request");
            return;
        }
        query.push_str(&format!(" WHERE id = {}", tid));

        let f = self
            .simple_query_one::<Talk>(query.as_str())
            .into_actor(self)
            .then(move |r, act, _| {
                match r {
                    Ok(t) => {
                        let s = serde_json::to_string(&t)
                            .unwrap_or("!!! Stringify Error.But admin query success".to_owned());
                        act.insert_talk(t);
                        act.send_message(&id, &s);
                    }
                    Err(e) => act.parse_send_res_error(id, &e)
                };
                fut::ok(())
            });
        ctx.spawn(f);
    }
}

impl Handler<DeleteTalkRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: DeleteTalkRequest, ctx: &mut Context<Self>) {
        if let Some(_) = self.get_talk(&msg.talk_id) {
            let session_id = msg.session_id.unwrap();

//ToDo: delete talk table and messages here.
            let query = format!("
                        DELETE FROM talks
                        WHERE id = {}", msg.talk_id);

            let f = self.simple_query_row(query.as_str())
                .into_actor(self)
                .map_err(move |_, act, _| {
                    act.send_message(&session_id, "!!! Database Error")
                })
                .and_then(move |_, act, _| {
                    let string = if act.remove_talk(&msg.talk_id).is_ok() {
                        "!! Delete talk success"
                    } else {
                        "!!! Talk not found in hash map. But delete is success."
                    };
                    act.send_message(&session_id, string);
                    fut::ok(())
                });

            ctx.spawn(f);
        }
    }
}