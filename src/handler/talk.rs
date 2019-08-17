use std::fmt::Write;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use actix::prelude::{fut, ActorFuture, Addr, AsyncContext, Context, Handler, Message, WrapFuture};
use chrono::{NaiveDateTime, Utc};
use futures::{
    future::{err as ft_err, Either},
    Future,
};
use hashbrown::HashMap;

use crate::handler::db::{Query, SimpleQuery};
use crate::model::{
    actors::{TalkService, WsChatSession},
    errors::ResError,
    talk::{PrivateMessage, PublicMessage, Relation, SendMessage, SessionMessage, Talk},
};

impl TalkService {
    // ToDo: add online offline filter
    fn send_message_many(&mut self, sid: &u32, tid: &u32, msg: &str) {
        match self.get_talk_hm(tid) {
            Ok(t) => {
                for u in t.users.iter() {
                    self.send_message(u, msg);
                }
            }
            Err(e) => self.parse_send_res_error(sid, &e),
        }
    }

    fn send_message(&self, sid: &u32, msg: &str) {
        match self.get_session_hm(sid) {
            Ok(a) => a.do_send(SessionMessage(msg.to_owned())),
            Err(e) => self.parse_send_res_error(sid, &e),
        };
    }

    fn parse_send_res_error(&self, sid: &u32, e: &ResError) {
        if let Some(addr) = self.get_session_hm(&sid).ok() {
            let _ = addr.do_send(SessionMessage(
                SendMessage::Error(e.stringify()).stringify(),
            ));
        }
    }

    fn get_talks_hm(&self) -> Result<HashMap<u32, Talk>, ResError> {
        self.read_talks(|t| Ok(t.clone()))
    }

    fn get_talk_hm(&self, talk_id: &u32) -> Result<Talk, ResError> {
        self.read_talks(|t| t.get(talk_id).cloned().ok_or(ResError::NotFound))
    }

    fn get_session_hm(&self, sid: &u32) -> Result<Addr<WsChatSession>, ResError> {
        self.read_sessions(|s| s.get(sid).cloned().ok_or(ResError::NotFound))
    }

    fn insert_talk_hm(&self, talk: Talk) -> Result<(), ResError> {
        self.write_talks(|mut t| {
            t.insert(talk.id, talk)
                .map(|_| ())
                .ok_or(ResError::NotFound)
        })
    }

    fn insert_session_hm(&self, sid: u32, addr: Addr<WsChatSession>) -> Result<(), ResError> {
        self.write_sessions(|mut s| s.insert(sid, addr).map(|_| ()).ok_or(ResError::NotFound))
    }

    fn insert_user_hm(&self, sid: &u32, tid: &u32) -> Result<(), ResError> {
        self.write_talks(|mut t| {
            t.get_mut(tid)
                .map(|t| t.users.push(*sid))
                .ok_or(ResError::NotFound)
        })
    }

    fn remove_talk_hm(&self, tid: &u32) -> Result<(), ResError> {
        self.write_talks(|mut t| t.remove(tid).map(|_| ()).ok_or(ResError::NotFound))
    }

    fn remove_session_hm(&self, sid: &u32) -> Result<(), ResError> {
        self.write_sessions(|mut s| s.remove(sid).map(|_| ()).ok_or(ResError::NotFound))
    }

    fn read_sessions<F, T>(&self, f: F) -> Result<T, ResError>
    where
        F: FnOnce(RwLockReadGuard<HashMap<u32, Addr<WsChatSession>>>) -> Result<T, ResError>,
    {
        self.sessions
            .try_read()
            .map_err(|_| ResError::InternalServerError)
            .and_then(|t| f(t))
    }

    fn read_talks<F, T>(&self, f: F) -> Result<T, ResError>
    where
        F: FnOnce(RwLockReadGuard<HashMap<u32, Talk>>) -> Result<T, ResError>,
    {
        self.talks
            .try_read()
            .map_err(|_| ResError::InternalServerError)
            .and_then(|t| f(t))
    }

    fn write_sessions<F>(&self, f: F) -> Result<(), ResError>
    where
        F: FnOnce(RwLockWriteGuard<HashMap<u32, Addr<WsChatSession>>>) -> Result<(), ResError>,
    {
        self.sessions
            .try_write()
            .map_err(|_| ResError::InternalServerError)
            .and_then(|t| f(t))
    }

    fn write_talks<F>(&self, f: F) -> Result<(), ResError>
    where
        F: FnOnce(RwLockWriteGuard<HashMap<u32, Talk>>) -> Result<(), ResError>,
    {
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

#[derive(Message)]
pub struct DisconnectRequest {
    pub session_id: u32,
}

impl TalkService {
    fn join_talk_db(&self, req: JoinTalkRequest) -> impl Future<Item = Talk, Error = ResError> {
        let sid = req.session_id.as_ref().unwrap();
        let tid = req.talk_id;
        match self.get_talk_hm(&tid) {
            Ok(t) => {
                if t.users.contains(sid) {
                    return Either::A(ft_err(ResError::BadRequest));
                };
                Either::B(self.query_one_trait::<Talk>(&self.join_talk, &[&sid, &tid]))
            }
            Err(e) => Either::A(ft_err(e)),
        }
    }

    fn get_relation(&self, uid: &u32) -> impl Future<Item = Relation, Error = ResError> {
        self.query_one_trait(&self.get_relations, &[uid])
    }

    fn insert_talk_db(
        &self,
        last_tid: u32,
        msg: &CreateTalkRequest,
    ) -> impl Future<Item = Talk, Error = ResError> {
        let query = format!(
            "INSERT INTO talks
            (id, name, description, owner, admin, users)
            VALUES ({}, '{}', '{}', {}, ARRAY [{}], ARRAY [{}])
             RETURNING *",
            (last_tid + 1),
            msg.name,
            msg.description,
            msg.owner,
            msg.owner,
            msg.owner
        );

        self.simple_query_one_trait(query.as_str())
    }

    fn get_last_tid_db(&self) -> impl Future<Item = u32, Error = ResError> {
        self.simple_query_single_row_trait::<u32>("SELECT Max(id) FROM talks", 0)
    }
}

impl Handler<DisconnectRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: DisconnectRequest, ctx: &mut Context<Self>) {
        let sid = msg.session_id;

        let _ = self
            .remove_session_hm(&sid)
            .map_err(|e| self.parse_send_res_error(&sid, &e))
            .map(|_| {
                ctx.spawn(
                    self.set_online_status(sid, 0, true)
                        .into_actor(self)
                        .map_err(move |e, act, _| act.parse_send_res_error(&sid, &e))
                        .map(|_, _, _| ()),
                )
            });
    }
}

impl Handler<TextMessageRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: TextMessageRequest, ctx: &mut Context<Self>) {
        // ToDo: batch insert messages to database.
        let sid = msg.session_id.unwrap();
        let now = Utc::now().naive_utc();

        if let Some(tid) = msg.talk_id {
            ctx.spawn(
                self.query_one_trait::<PublicMessage>(
                    &self.insert_pub_msg,
                    &[&tid, &msg.text, &now],
                )
                .into_actor(self)
                .map_err(move |e, act, _| act.parse_send_res_error(&sid, &e))
                .map(move |_, act, _| {
                    let s = SendMessage::PublicMessage(&vec![PublicMessage {
                        text: msg.text,
                        time: now,
                        talk_id: msg.talk_id.unwrap(),
                    }])
                    .stringify();

                    act.send_message_many(&sid, &tid, s.as_str());
                }),
            );
            return;
        }

        if let Some(uid) = msg.user_id {
            ctx.spawn(
                self.query_one_trait::<PrivateMessage>(
                    &self.insert_prv_msg,
                    &[&msg.session_id.unwrap(), &uid, &msg.text, &now],
                )
                .into_actor(self)
                .map_err(move |e, act, _| act.parse_send_res_error(&sid, &e))
                .map(move |_, act, _| {
                    let s = SendMessage::PrivateMessage(&vec![PrivateMessage {
                        user_id: msg.user_id.unwrap(),
                        text: msg.text,
                        time: now,
                    }])
                    .stringify();

                    act.send_message(&uid, s.as_str());
                }),
            );
        }
    }
}

impl Handler<ConnectRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: ConnectRequest, ctx: &mut Context<Self>) {
        let sid = msg.session_id;

        ctx.spawn(
            self.set_online_status(sid, msg.online_status, true)
                .into_actor(self)
                .map_err(move |e, act, _| act.parse_send_res_error(&sid, &e))
                .map(move |_, act, _| {
                    let _ = act
                        .insert_session_hm(sid, msg.addr.clone())
                        .map_err(|e| act.parse_send_res_error(&sid, &e))
                        .map(|_| {
                            let _ = msg.addr.do_send(SessionMessage(
                                SendMessage::Success("Connection Success").stringify(),
                            ));
                        });
                }),
        );
    }
}

impl Handler<CreateTalkRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: CreateTalkRequest, ctx: &mut Context<Self>) {
        let sid = msg.session_id.unwrap();

        ctx.spawn(
            self.get_last_tid_db()
                .into_actor(self)
                .map_err(move |e, act, _| act.parse_send_res_error(&sid, &e))
                .and_then(move |tid, act, _| {
                    act.insert_talk_db(tid, &msg)
                        .into_actor(act)
                        .map_err(move |e, act, _| act.parse_send_res_error(&sid, &e))
                        .map(move |t, act, _| {
                            let s = SendMessage::Talks(vec![&t]).stringify();
                            let _ = act
                                .insert_talk_hm(t)
                                .map_err(|e| act.parse_send_res_error(&sid, &e))
                                .map(|_| {
                                    let _ = act.send_message(&msg.owner, s.as_str());
                                });
                        })
                }),
        );
    }
}

impl Handler<JoinTalkRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: JoinTalkRequest, ctx: &mut Context<Self>) {
        let sid = msg.session_id.unwrap();
        let tid = msg.talk_id;

        ctx.spawn(
            self.join_talk_db(msg)
                .into_actor(self)
                .map_err(move |e, act, _| act.parse_send_res_error(&sid, &e))
                .map(move |t, act, _| {
                    let s = SendMessage::Talks(vec![&t]).stringify();
                    let _ = act
                        .insert_user_hm(&sid, &tid)
                        .map_err(|e| act.parse_send_res_error(&sid, &e))
                        .map(|_| act.send_message(&sid, s.as_str()));
                }),
        );
    }
}

#[derive(Message, Deserialize)]
pub struct TalkByIdRequest {
    pub session_id: Option<u32>,
    pub talk_id: u32,
}

impl Handler<TalkByIdRequest> for TalkService {
    type Result = ();
    fn handle(&mut self, msg: TalkByIdRequest, _: &mut Context<Self>) {
        let sid = msg.session_id.as_ref().unwrap();

        let _ = self
            .get_talks_hm()
            .map_err(|e| self.parse_send_res_error(sid, &e))
            .map(|t| {
                let t = match msg.talk_id {
                    0 => t.iter().map(|(_, t)| t).collect(),
                    _ => t
                        .get(&msg.talk_id)
                        .map(|t| vec![t])
                        .unwrap_or_else(|| vec![]),
                };
                self.send_message(sid, SendMessage::Talks(t).stringify().as_str())
            });
    }
}

#[derive(Message, Deserialize)]
pub struct UsersByIdRequest {
    pub session_id: Option<u32>,
    user_id: Vec<u32>,
}

impl Handler<UsersByIdRequest> for TalkService {
    type Result = ();
    fn handle(&mut self, msg: UsersByIdRequest, ctx: &mut Context<Self>) {
        let sid = msg.session_id.unwrap();

        ctx.spawn(
            self.get_users_cache_from_ids(msg.user_id)
                .into_actor(self)
                .map_err(move |e, act, _| act.parse_send_res_error(&sid, &e))
                .map(move |u, act, _| {
                    let s = SendMessage::Users(&u).stringify();
                    act.send_message(&sid, s.as_str())
                }),
        );
    }
}

impl Handler<UserRelationRequest> for TalkService {
    type Result = ();
    fn handle(&mut self, msg: UserRelationRequest, ctx: &mut Context<Self>) {
        let sid = msg.session_id.unwrap();

        ctx.spawn(
            self.get_relation(&sid)
                .into_actor(self)
                .map_err(move |e, act, _| act.parse_send_res_error(&sid, &e))
                .map(move |r, act, _| {
                    let _ = act
                        .send_message(&sid, SendMessage::Friends(&r.friends).stringify().as_str());
                }),
        );
    }
}

impl Handler<GetHistory> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: GetHistory, ctx: &mut Context<Self>) {
        let sid = msg.session_id.unwrap();
        let time = NaiveDateTime::parse_from_str(&msg.time, "%Y-%m-%d %H:%M:%S%.f")
            .unwrap_or_else(|_| Utc::now().naive_utc());

        match msg.talk_id {
            Some(tid) => ctx.spawn(
                self.get_by_time::<PublicMessage>(&self.get_pub_msg, &[&tid, &time])
                    .into_actor(self)
                    .map_err(move |e, act, _| act.parse_send_res_error(&sid, &e))
                    .map(move |m, act, _| {
                        let s = SendMessage::PublicMessage(&m).stringify();
                        act.send_message(&sid, s.as_str())
                    }),
            ),
            None => ctx.spawn(
                self.get_by_time::<PrivateMessage>(&self.get_prv_msg, &[&sid, &time])
                    .into_actor(self)
                    .map_err(move |e, act, _| act.parse_send_res_error(&sid, &e))
                    .map(move |m, act, _| {
                        let s = SendMessage::PrivateMessage(&m).stringify();
                        act.send_message(&sid, s.as_str())
                    }),
            ),
        };
    }
}

impl Handler<RemoveUserRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: RemoveUserRequest, ctx: &mut Context<Self>) {
        let sid = msg.session_id.unwrap();
        let tid = msg.talk_id;
        let uid = msg.user_id;

        match self.get_session_hm(&sid) {
            Err(e) => self.parse_send_res_error(&sid, &e),
            Ok(addr) => match self.get_talk_hm(&tid) {
                Err(e) => self.parse_send_res_error(&sid, &e),
                Ok(talk) => {
                    if !talk.users.contains(&uid) {
                        let _ =
                            addr.do_send(SessionMessage("!!! User not found in talk".to_owned()));
                        return;
                    }

                    let other_is_admin = talk.admin.contains(&uid);
                    let other_is_owner = talk.owner == uid;
                    let self_is_admin = talk.admin.contains(&sid);
                    let self_is_owner = talk.owner == sid;

                    let query = if self_is_owner && other_is_admin {
                        format!("UPDATE talks SET admin=array_remove(admin, {}), users=array_remove(users, {})
                        WHERE id={} AND owner={}", uid, uid, tid, sid)
                    } else if (self_is_admin || self_is_owner) && !other_is_admin && !other_is_owner
                    {
                        format!(
                            "UPDATE talks SET users=array_remove(users, {})
                        WHERE id={}",
                            uid, tid
                        )
                    } else {
                        let _ = addr.do_send(SessionMessage("!!! Unauthorized".to_owned()));
                        return;
                    };

                    ctx.spawn(
                        self.simple_query_one_trait::<Talk>(query.as_str())
                            .into_actor(self)
                            .then(move |r, act, _| {
                                match r {
                                    Ok(t) => {
                                        let s = serde_json::to_string(&t).unwrap_or(
                                            "!!! Stringify Error.But user removal success"
                                                .to_owned(),
                                        );

                                        match act.insert_talk_hm(t) {
                                            Ok(_) => act.send_message_many(&sid, &tid, &s),
                                            Err(e) => act.parse_send_res_error(&sid, &e),
                                        };
                                    }
                                    Err(e) => {
                                        let _ =
                                            addr.do_send(SessionMessage(e.stringify().to_owned()));
                                    }
                                }
                                fut::ok(())
                            }),
                    );
                }
            },
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

        query.push_str(&format!(" WHERE id = {}", tid));

        ctx.spawn(
            self.simple_query_one_trait::<Talk>(query.as_str())
                .into_actor(self)
                .map_err(move |e, act, _| act.parse_send_res_error(&sid, &e))
                .map(move |t, act, _| {
                    let s = SendMessage::Talks(vec![&t]).stringify();
                    let _ = act
                        .insert_talk_hm(t)
                        .map_err(|e| act.parse_send_res_error(&sid, &e))
                        .map(|_| act.send_message(&sid, s.as_str()));
                }),
        );
    }
}

impl Handler<DeleteTalkRequest> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: DeleteTalkRequest, ctx: &mut Context<Self>) {
        let tid = msg.talk_id;

        if self.get_talk_hm(&tid).ok().is_some() {
            let sid = msg.session_id.unwrap();
            //ToDo: delete talk table and messages here.
            let query = format!("DELETE FROM talks WHERE id = {}", tid);

            ctx.spawn(
                self.simple_query_row_trait(query.as_str())
                    .into_actor(self)
                    .map_err(move |e, act, _| act.parse_send_res_error(&sid, &e))
                    .map(move |_, act, _| {
                        let _ = act
                            .remove_talk_hm(&msg.talk_id)
                            .map_err(|e| act.parse_send_res_error(&sid, &e))
                            .map(|_| {
                                act.send_message(
                                    &sid,
                                    SendMessage::Success("Delete Talk Success")
                                        .stringify()
                                        .as_str(),
                                )
                            });
                    }),
            );
        }
    }
}
