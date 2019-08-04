use std::fmt::Write;
use std::sync::{RwLockWriteGuard, RwLockReadGuard};

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
    actors::WsChatSession,
    errors::ResError,
    user::User,
    talk::{Talk, PublicMessage, PrivateMessage, SessionMessage},
};
use crate::model::actors::TalkService;
use crate::handler::db::{Query, SimpleQuery};
use crate::model::talk::Relation;

impl TalkService {
    // ToDo: add online offline filter
    fn send_message_many(&mut self, sid: &u32, tid: &u32, msg: &str) {
        match self.get_talk(tid) {
            Ok(t) => for u in t.users.iter() {
                self.send_message(u, msg);
            },
            Err(e) => self.parse_send_res_error(sid, &e)
        }
    }

    fn send_message(&self, sid: &u32, msg: &str) {
        match self.get_session(sid) {
            Ok(a) => a.do_send(SessionMessage(msg.to_owned())),
            Err(e) => self.parse_send_res_error(sid, &e)
        };
    }

    fn parse_send_res_error(&self, sid: &u32, e: &ResError) {
        if let Some(addr) = self.get_session(&sid).ok() {
            let _ = addr.do_send(SessionMessage(SendMessage::Error(e.stringify()).stringify()));
        }
    }

    fn get_talks(&self) -> Result<HashMap<u32, Talk>, ResError> {
        self.read_talks(|t| Ok(t.clone()))
    }

    fn get_talk(&self, talk_id: &u32) -> Result<Talk, ResError> {
        self.read_talks(|t| t
            .get(talk_id)
            .map(|t| t.clone())
            .ok_or(ResError::NotFound))
    }

    fn get_session(&self, sid: &u32) -> Result<Addr<WsChatSession>, ResError> {
        self.read_sessions(|s| s
            .get(sid)
            .map(|addr| addr.clone())
            .ok_or(ResError::NotFound)
        )
    }

    fn insert_talk(&self, talk: Talk) -> Result<(), ResError> {
        self.write_talks(|mut t| t
            .insert(talk.id, talk)
            .map(|_| ())
            .ok_or(ResError::NotFound)
        )
    }

    fn remove_talk(&self, tid: &u32) -> Result<(), ResError> {
        self.write_talks(|mut t| t
            .remove(tid)
            .map(|_| ())
            .ok_or(ResError::NotFound)
        )
    }

    fn insert_user(&self, sid: &u32, tid: &u32) -> Result<(), ResError> {
        self.write_talks(|mut t| t
            .get_mut(tid)
            .map(|t| t.users.push(*sid))
            .ok_or(ResError::NotFound)
        )
    }

    fn read_sessions<F, T>(&self, f: F) -> Result<T, ResError>
        where F: FnOnce(RwLockReadGuard<HashMap<u32, Addr<WsChatSession>>>) -> Result<T, ResError> {
        self.sessions
            .try_read()
            .map_err(|_| ResError::InternalServerError)
            .and_then(|t| f(t))
    }

    fn read_talks<F, T>(&self, f: F) -> Result<T, ResError>
        where F: FnOnce(RwLockReadGuard<HashMap<u32, Talk>>) -> Result<T, ResError> {
        self.talks
            .try_read()
            .map_err(|_| ResError::InternalServerError)
            .and_then(|t| f(t))
    }

    fn write_sessions<F>(&self, f: F) -> Result<(), ResError>
        where F: FnOnce(RwLockWriteGuard<HashMap<u32, Addr<WsChatSession>>>) -> Result<(), ResError> {
        self.sessions
            .try_write()
            .map_err(|_| ResError::InternalServerError)
            .and_then(|t| f(t))
    }

    fn write_talks<F>(&self, f: F) -> Result<(), ResError>
        where F: FnOnce(RwLockWriteGuard<HashMap<u32, Talk>>) -> Result<(), ResError> {
        self.talks
            .try_write()
            .map_err(|_| ResError::InternalServerError)
            .and_then(|t| f(t))
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
        let sid = msg.session_id;

        let _ = self
            .write_sessions(|mut t| t
                .remove(&sid)
                .map(|_| ())
                .ok_or(ResError::InternalServerError)
            )
            .map_err(|e| self.parse_send_res_error(&sid, &e))
            .map(|_| {
                ctx.spawn(self
                    .set_online_status(sid, 0, true)
                    .into_actor(self)
                    .map_err(move |e, act, _| act
                        .parse_send_res_error(&sid, &e)
                    )
                    .map(|_, _, _| ()));
            });
    }
}

impl Handler<TextMessageRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: TextMessageRequest, ctx: &mut Context<Self>) {
// ToDo: batch insert messages to database.
        let sid = msg.session_id.unwrap();

        if let Some(tid) = msg.talk_id {
            let now = Utc::now().naive_utc();
            ctx.spawn(self
                .query_one_trait::<PublicMessage>(
                    &self.insert_pub_msg,
                    &[&tid, &msg.text, &now])
                .into_actor(self)
                .map_err(move |e, act, _|
                    act.parse_send_res_error(&sid, &e)
                )
                .map(move |_, act, _| {
                    let s = SendMessage::PublicMessage(
                        &vec![PublicMessage {
                            text: msg.text,
                            time: now,
                            talk_id: msg.talk_id.unwrap(),
                        }]
                    ).stringify();

                    act.send_message_many(&sid, &tid, s.as_str());
                }));
            return;
        }

        if let Some(uid) = msg.user_id {
            let now = Utc::now().naive_utc();
            ctx.spawn(self
                .query_one_trait::<PrivateMessage>(
                    &self.insert_prv_msg,
                    &[&msg.session_id.unwrap(), &uid, &msg.text, &now])
                .into_actor(self)
                .map_err(move |e, act, _|
                    act.parse_send_res_error(&sid, &e)
                )
                .map(move |_, act, _| {
                    let s = SendMessage::PrivateMessage(&vec![
                        PrivateMessage {
                            user_id: msg.user_id.unwrap(),
                            text: msg.text,
                            time: now,
                        }]).stringify();

                    act.send_message(&uid, s.as_str());
                }));
        }
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
                    Err(e) => act.parse_send_res_error(&msg.session_id, &e)
                };
                fut::ok(())
            });
        ctx.spawn(f);
    }
}

impl Handler<CreateTalkRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: CreateTalkRequest, ctx: &mut Context<Self>) {
        let sid = msg.session_id.unwrap();

        let f = self
            .simple_query_single_row_trait::<u32>("SELECT Max(id) FROM talks", 0)
            .into_actor(self)
            // ToDo: handle error.
            .map_err(move |e, act, _| act.parse_send_res_error(&sid, &e))
            .and_then(move |cid, act, _| {
                //ToDo: in case query array failed.
                let query = format!("
                    INSERT INTO talks
                    (id, name, description, owner, admin, users)
                    VALUES ({}, '{}', '{}', {}, ARRAY [{}], ARRAY [{}])
                    RETURNING *", cid, msg.name, msg.description, msg.owner, cid, cid);

                act.simple_query_one_trait(query.as_str())
                    .into_actor(act)
                    .map_err(move |e, act, _| act.parse_send_res_error(&sid, &e))
                    .map(move |t, act, _| {
                        let s = SendMessage::Talks(vec![&t]).stringify();
                        let _ = act
                            .insert_talk(t)
                            .map_err(|e| act.parse_send_res_error(&sid, &e))
                            .map(|_| {
                                let _ = act.send_message(&msg.owner, s.as_str());
                            });
                    })
            });
        ctx.spawn(f);
    }
}

impl Handler<JoinTalkRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: JoinTalkRequest, ctx: &mut Context<Self>) {
        let sid = msg.session_id.unwrap();
        let tid = msg.talk_id;

        match self.get_talk(&tid) {
            Ok(t) => {
                if t.users.contains(&sid) {
                    self.send_message(&sid, SendMessage::Error("Already Joined").stringify().as_str());
                    return;
                };

                ctx.spawn(self
                    .query_one_trait::<Talk>(
                        &self.join_talk,
                        &[&sid, &tid])
                    .into_actor(self)
                    .map_err(move |e, act, _| act.parse_send_res_error(&sid, &e))
                    .map(move |_, act, _| {
                        let _ = act
                            .insert_user(&sid, &tid)
                            .map_err(|e| act.parse_send_res_error(&sid, &e))
                            .map(|_| act.send_message(&sid, SendMessage::Success("Join Talk Success").stringify().as_str()));
                    })
                );
            }
            Err(e) => self.parse_send_res_error(&sid, &e)
        }
    }
}

impl Handler<TalkByIdRequest> for TalkService {
    type Result = ();
    fn handle(&mut self, msg: TalkByIdRequest, _: &mut Context<Self>) {
        match self.get_talks() {
            Ok(t) => {
                let t = match msg.talk_id {
                    0 => t.iter().map(|(_, t)| t).collect(),
                    _ => t.get(&msg.talk_id).map(|t| vec![t]).unwrap_or(vec![])
                };

                self.send_message(&msg.session_id.unwrap(), SendMessage::Talks(t).stringify().as_str());
            }
            Err(e) => self.parse_send_res_error(msg.session_id.as_ref().unwrap(), &e)
        }
    }
}

impl Handler<UsersByIdRequest> for TalkService {
    type Result = ();
    fn handle(&mut self, msg: UsersByIdRequest, ctx: &mut Context<Self>) {
        let sid = msg.session_id.unwrap();

        match self.get_session(&sid) {
            Err(e) => self.parse_send_res_error(&sid, &e),
            Ok(addr) => {
                ctx.spawn(self
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
                    }));
            }
        }
    }
}

impl Handler<UserRelationRequest> for TalkService {
    type Result = ();
    fn handle(&mut self, msg: UserRelationRequest, ctx: &mut Context<Self>) {
        let f = self
            .query_one_trait(
                &self.get_relations,
                &[msg.session_id.as_ref().unwrap()],
            )
            .into_actor(self)
            .then(move |r: Result<Relation, ResError>, act, _| {
                match r {
                    Ok(r) => act.send_message(
                        msg.session_id.as_ref().unwrap(),
                        SendMessage::Friends(&r.friends).stringify().as_str()),
                    Err(e) => act.parse_send_res_error(msg.session_id.as_ref().unwrap(), &e)
                };
                fut::ok(())
            });
        ctx.spawn(f);
    }
}

impl Handler<GetHistory> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: GetHistory, ctx: &mut Context<Self>) {
        let sid = msg.session_id.unwrap();

        match self.get_session(&sid) {
            Err(e) => self.parse_send_res_error(&sid, &e),
            Ok(addr) => {
                let time = NaiveDateTime::parse_from_str(&msg.time, "%Y-%m-%d %H:%M:%S%.f")
                    .unwrap_or(Utc::now().naive_utc());
                match msg.talk_id {
                    Some(tid) => {
                        let f = self
                            .get_by_time(
                                &self.get_pub_msg,
                                &[&tid, &time],
                            )
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
}

impl Handler<RemoveUserRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: RemoveUserRequest, ctx: &mut Context<Self>) {
        let sid = msg.session_id.unwrap();
        let tid = msg.talk_id;
        let uid = msg.user_id;

        match self.get_session(&sid) {
            Err(e) => self.parse_send_res_error(&sid, &e),
            Ok(addr) => match self.get_talk(&tid) {
                Err(e) => self.parse_send_res_error(&sid, &e),
                Ok(talk) => {
                    if !talk.users.contains(&uid) {
                        let _ = addr.do_send(SessionMessage("!!! User not found in talk".to_owned()));
                        return;
                    }

                    let other_is_admin = talk.admin.contains(&uid);
                    let other_is_owner = talk.owner == uid;
                    let self_is_admin = talk.admin.contains(&sid);
                    let self_is_owner = talk.owner == sid;

                    let query = if self_is_owner && other_is_admin {
                        format!("UPDATE talks SET admin=array_remove(admin, {}), users=array_remove(users, {})
                        WHERE id={} AND owner={}", uid, uid, tid, sid)
                    } else if (self_is_admin || self_is_owner) && !other_is_admin && !other_is_owner {
                        format!("UPDATE talks SET users=array_remove(users, {})
                        WHERE id={}", uid, tid)
                    } else {
                        let _ = addr.do_send(SessionMessage("!!! Unauthorized".to_owned()));
                        return;
                    };

                    ctx.spawn(self
                        .simple_query_one_trait::<Talk>(query.as_str())
                        .into_actor(self)
                        .then(move |r, act, _| {
                            match r {
                                Ok(t) => {
                                    let s = serde_json::to_string(&t)
                                        .unwrap_or("!!! Stringify Error.But user removal success".to_owned());

                                    match act.insert_talk(t) {
                                        Ok(_) => act.send_message_many(&sid, &tid, &s),
                                        Err(e) => act.parse_send_res_error(&sid, &e)
                                    };
                                }
                                Err(e) => {
                                    let _ = addr.do_send(SessionMessage(e.stringify().to_owned()));
                                }
                            }
                            fut::ok(())
                        }));
                }
            }
        }
    }
}

impl Handler<Admin> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Admin, ctx: &mut Context<Self>) {
        let tid = msg.talk_id;
        let sid = msg.session_id.unwrap();

        let mut query = "UPDATE talks SET admin=".to_owned();

        if let Some(uid) = msg.add {
            let _ = write!(&mut query, "array_append(admin, {})", uid);
        }

        if let Some(uid) = msg.remove {
            let _ = write!(&mut query, "array_remove(admin, {})", uid);
        }

        if query.ends_with("=") {
            self.send_message(&sid, "!!! Empty request");
            return;
        }
        query.push_str(&format!(" WHERE id = {}", tid));

        let f = self
            .simple_query_one_trait::<Talk>(query.as_str())
            .into_actor(self)
            .then(move |r, act, _| {
                match r {
                    Ok(t) => {
                        let s = SendMessage::Talks(vec![&t]).stringify();
                        let _ = act
                            .insert_talk(t)
                            .map_err(|e| act.parse_send_res_error(&sid, &e))
                            .map(|_| act.send_message(&sid, s.as_str()));
                    }
                    Err(e) => act.parse_send_res_error(&sid, &e)
                };
                fut::ok(())
            });
        ctx.spawn(f);
    }
}

impl Handler<DeleteTalkRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: DeleteTalkRequest, ctx: &mut Context<Self>) {
        let tid = msg.talk_id;

        if let Some(_) = self.get_talk(&tid).ok() {
            let sid = msg.session_id.unwrap();

            //ToDo: delete talk table and messages here.
            let query = format!("
                        DELETE FROM talks
                        WHERE id = {}", tid);

            ctx.spawn(self
                .simple_query_row_trait(query.as_str())
                .into_actor(self)
                .then(move |r: Result<_, ResError>, act, _| {
                    match r {
                        Ok(_) => {
                            let _ = act
                                .remove_talk(&msg.talk_id)
                                .map_err(|e| act.parse_send_res_error(&sid, &e))
                                .map(|_| act.send_message(&sid, SendMessage::Success("Delete Talk Success").stringify().as_str()));
                        }
                        Err(e) => act.parse_send_res_error(&sid, &e)
                    };
                    fut::ok(())
                }));
        }
    }
}