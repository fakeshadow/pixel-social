use std::collections::HashMap;
use std::fmt::Write;
use futures::{Future, future::{err as ft_err, ok as ft_ok}, IntoFuture};

use actix::prelude::*;
use chrono::{NaiveDateTime, Utc};

use crate::model::{
    actors::TalkService,
    errors::ServiceError,
    user::User,
    talk::{Talk, SessionMessage},
};
use crate::handler::{
    db::{get_single_row, simple_query, query_talk},
    cache::get_users,
};

impl TalkService {
    fn send_message_many(&self, id: u32, msg: &str) {
        if let Some(talk) = self.talks.get(&id) {
            talk.users.iter().for_each(|id| self.send_message(id, msg));
        }
    }

    fn send_message(&self, session_id: &u32, msg: &str) {
        if let Some(addr) = self.sessions.get(&session_id) {
            let _ = addr.do_send(SessionMessage(msg.to_owned()));
        }
    }
}

#[derive(Serialize)]
struct HistoryMessage {
    pub talk_id: u32,
    pub time: NaiveDateTime,
    pub message: String,
}

#[derive(Message)]
pub struct Connect {
    pub session_id: u32,
    pub addr: Recipient<SessionMessage>,
}

#[derive(Message)]
pub struct Disconnect {
    pub session_id: u32,
}

#[derive(Deserialize, Clone)]
pub struct Create {
    pub name: String,
    pub description: String,
    pub owner: u32,
}

pub struct Join {
    pub session_id: u32,
    pub talk_id: u32,
}

#[derive(Deserialize)]
pub struct RemoveUser {
    pub session_id: u32,
    user_id: u32,
    talk_id: u32,
}

impl Message for RemoveUser {
    type Result = Result<(), ServiceError>;
}

#[derive(Message)]
pub struct GetTalks {
    pub session_id: u32,
    pub talk_id: u32,
}

// pass Some(talk_id) in json for public message, pass None for private message
#[derive(Deserialize)]
pub struct ClientMessage {
    pub msg: String,
    pub talk_id: Option<u32>,
    pub session_id: u32,
}

pub struct GetTalkUsers {
    pub session_id: u32,
    pub talk_id: u32,
}

// pass talk id for talk public messages. pass none for private history message.
#[derive(Deserialize)]
pub struct GetHistory {
    pub time: String,
    pub talk_id: Option<u32>,
    pub session_id: u32,
}

#[derive(Deserialize)]
pub struct Admin {
    pub add: Option<u32>,
    pub remove: Option<u32>,
    pub talk_id: u32,
    pub session_id: u32,
}

impl Message for Create {
    type Result = Result<(), ServiceError>;
}

impl Message for Join {
    type Result = Result<(), ServiceError>;
}

impl Message for ClientMessage {
    type Result = Result<(), ServiceError>;
}

impl Message for GetTalkUsers {
    type Result = Result<(), ServiceError>;
}

impl Message for GetHistory {
    type Result = Result<(), ServiceError>;
}

impl Message for Admin {
    type Result = Result<(), ServiceError>;
}

impl Handler<Disconnect> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.sessions.remove(&msg.session_id);
    }
}

impl Handler<ClientMessage> for TalkService {
    type Result = ResponseFuture<(), ServiceError>;

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) -> Self::Result {
        // ToDo: batch insert messages to database.
        match msg.talk_id {
            Some(id) => {
                let _ = self.send_message_many(id, &msg.msg);
                let f = self.db
                    .as_mut()
                    .unwrap()
                    .query(self.insert_pub_msg.as_ref().unwrap(), &[&id, &msg.msg])
                    .into_future()
                    .from_err()
                    .map(|_| ());

                Box::new(f)
            }
            None => {
                let _ = self.send_message(&msg.session_id, &msg.msg);
                // ToDo: add private message insert statement
                let f = self.db
                    .as_mut()
                    .unwrap()
                    .query(self.insert_pub_msg.as_ref().unwrap(), &[&1, &msg.msg])
                    .into_future()
                    .from_err()
                    .map(|_| ());

                Box::new(f)
            }
        }
    }
}

impl Handler<Connect> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        self.sessions.insert(msg.session_id, msg.addr);
        self.send_message(&msg.session_id, "Authentication success");
    }
}

impl Handler<Create> for TalkService {
    type Result = ResponseActFuture<Self, (), ServiceError>;

    fn handle(&mut self, msg: Create, _: &mut Context<Self>) -> Self::Result {
        let query = "SELECT Max(id) FROM talks";

        let f =
            get_single_row::<u32>(self.db.as_mut().unwrap(), query, 0)
                .into_actor(self)
                .and_then(move |cid, act, _| {
                    //ToDo: in case query array failed.
                    let query = format!("
                    INSERT INTO talks
                    (id, name, description, owner, admin, users)
                    VALUES ({}, '{}', '{}', {}, ARRAY [{}], ARRAY [{}])
                    RETURNING *", cid, msg.name, msg.description, msg.owner, cid, cid);

                    query_talk(act.db.as_mut().unwrap(), &query)
                        .into_actor(act)
                        .and_then(move |t, act, _| {
                            let s = serde_json::to_string(&t)
                                .unwrap_or("!!! Stringify Error. But Talk Creation is success".to_owned());
                            act.talks.insert(t.id, t);
                            act.send_message(&msg.owner, &s);
                            fut::ok(())
                        })
                });
        Box::new(f)
    }
}

impl Handler<Join> for TalkService {
    type Result = ResponseActFuture<Self, (), ServiceError>;

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) -> Self::Result {
        match self.talks.get(&msg.talk_id) {
            Some(talk) => {
                if talk.users.contains(&msg.session_id) {
                    self.send_message(&msg.session_id, "Already joined");
                    return Box::new(fut::err(ServiceError::BadRequest));
                };
                // ToDo: in case sql failed.

                let f = self.db
                    .as_mut()
                    .unwrap()
                    .query(self.join_talk.as_ref().unwrap(), &[&msg.session_id, &msg.talk_id])
                    .into_future()
                    .map_err(|e| e.0)
                    .from_err()
                    .into_actor(self)
                    .and_then(move |row, act, _| match row.0 {
                        Some(_) => {
                            act.talks.get_mut(&msg.talk_id).unwrap().users.push(msg.session_id);
                            act.send_message(&msg.session_id, "!! Joined");
                            fut::ok(())
                        }
                        None => {
                            act.send_message(&msg.session_id, "!!! Joined failed");
                            fut::ok(())
                        }
                    });

                Box::new(f)
            }
            None => {
                self.send_message(&msg.session_id, "!!! Talk not found");
                Box::new(fut::err(ServiceError::BadRequest))
            }
        }
    }
}

impl Handler<GetTalks> for TalkService {
    type Result = ();
    fn handle(&mut self, msg: GetTalks, _: &mut Context<Self>) {
        let talks = match msg.session_id {
            0 => self.talks.iter().map(|(_, t)| t).collect(),
            _ => self.talks.get(&msg.talk_id).map(|t| vec![t]).unwrap_or(vec![])
        };
        let string = serde_json::to_string(&talks).unwrap_or("!!! Stringify error".to_owned());
        self.send_message(&msg.session_id, &string);
    }
}

impl Handler<GetHistory> for TalkService {
    type Result = ResponseActFuture<Self, (), ServiceError>;

    fn handle(&mut self, msg: GetHistory, _: &mut Context<Self>) -> Self::Result {
        if let Some(_) = self.sessions.get(&msg.session_id) {
            let time = NaiveDateTime::parse_from_str(&msg.time, "%Y-%m-%d %H:%M:%S%.f")
                .unwrap_or(Utc::now().naive_local());

            let msgs = Vec::with_capacity(20);

            return match msg.talk_id {
                Some(tid) => {
                    let f = self.db
                        .as_mut()
                        .unwrap()
                        .query(self.get_pub_msg.as_ref().unwrap(), &[&tid, &time])
                        .from_err()
                        .fold(msgs, move |mut msgs, row| {
                            msgs.push(HistoryMessage {
                                talk_id: row.get(0),
                                time: row.get(1),
                                message: row.get(2),
                            });
                            Ok::<Vec<HistoryMessage>, ServiceError>(msgs)
                        })
                        .into_actor(self)
                        .and_then(move |h, act, _| {
                            let s = serde_json::to_string(&h).unwrap_or("!!! Stringify Error".to_owned());
                            act.send_message(&msg.session_id, &s);
                            fut::ok(())
                        });

                    Box::new(f)
                }
                // ToDo: add private message table and prepare statement
                None => {
                    let f = self.db
                        .as_mut()
                        .unwrap()
                        .query(self.get_pub_msg.as_ref().unwrap(), &[&1, &time])
                        .from_err()
                        .fold(msgs, move |mut msgs, row| {
                            msgs.push(HistoryMessage {
                                talk_id: row.get(0),
                                time: row.get(1),
                                message: row.get(2),
                            });
                            Ok::<Vec<HistoryMessage>, ServiceError>(msgs)
                        })
                        .into_actor(self)
                        .and_then(move |h, act, _| {
                            let s = serde_json::to_string(&h).unwrap_or("!!! Stringify Error".to_owned());
                            act.send_message(&msg.session_id, &s);
                            fut::ok(())
                        });
                    Box::new(f)
                }
            };
        }

        Box::new(fut::err(ServiceError::BadRequest))
    }
}

impl Handler<GetTalkUsers> for TalkService {
    type Result = ResponseActFuture<Self, (), ServiceError>;

    fn handle(&mut self, msg: GetTalkUsers, _: &mut Context<Self>) -> Self::Result {
        if let Some(_) = self.sessions.get(&msg.session_id) {
            if let Some(talk) = self.talks.get(&msg.talk_id) {
                let f = get_users(self.cache.as_ref().unwrap().clone(), talk.users.clone())
                    .into_actor(self)
                    .and_then(move |u, act, _| {
                        let string = serde_json::to_string(&u)
                            .unwrap_or("failed to serialize users".to_owned());

                        act.send_message(&msg.session_id, &string);
                        fut::ok(())
                    });

                return Box::new(f);
            }
            self.send_message(&msg.session_id, "!!! Bad request.Talk not found");
            return Box::new(fut::err(ServiceError::BadRequest));
        }
        self.send_message(&msg.session_id, "!!! Bad request.Session not found");
        Box::new(fut::err(ServiceError::BadRequest))
    }
}

impl Handler<RemoveUser> for TalkService {
    type Result = ResponseActFuture<Self, (), ServiceError>;

    fn handle(&mut self, msg: RemoveUser, _: &mut Context<Self>) -> Self::Result {
        let id = msg.session_id;
        let tid = msg.talk_id;
        let uid = msg.user_id;

        if let Some(talk) = self.talks.get(&tid) {
            if !talk.users.contains(&uid) {
                self.send_message(&id, "!!! Target user not found in talk");
                return Box::new(fut::err(ServiceError::BadRequest));
            }

            let other_is_admin = talk.admin.contains(&uid);
            let other_is_owner = talk.owner == uid;
            let is_admin = talk.admin.contains(&id);
            let is_owner = talk.owner == id;

            let query = if is_owner && other_is_admin {
                format!("UPDATE talks SET admin=array_remove(admin, {}), users=array_remove(users, {})
                WHERE id={} AND owner={}", uid, uid, tid, id)
            } else if (is_admin || is_owner) && !other_is_admin && !other_is_owner {
                format!("UPDATE talks SET users=array_remove(users, {})
                WHERE id={}", uid, tid)
            } else {
                self.send_message(&id, "!!! Unauthorized");
                return Box::new(fut::err(ServiceError::Unauthorized));
            };

            let f = query_talk(self.db.as_mut().unwrap(), &query)
                .into_actor(self)
                .and_then(move |t, act, _| {
                    let s = serde_json::to_string(&t).unwrap_or("!!! Stringify Error.But user removal success".to_owned());

                    act.talks.insert(t.id, t);
                    act.send_message_many(tid, &s);
                    fut::ok(())
                });
            return Box::new(f);
        }

        self.send_message(&id, "!!! Talk not found");
        Box::new(fut::err(ServiceError::BadRequest))
    }
}

impl Handler<Admin> for TalkService {
    type Result = ResponseActFuture<Self, (), ServiceError>;

    fn handle(&mut self, msg: Admin, _: &mut Context<Self>) -> Self::Result {
        let tid = msg.talk_id;
        let id = msg.session_id;

        match self.talks.get(&tid) {
            Some(t) => {
                let mut query = "UPDATE talks SET admin=".to_owned();

                if let Some(uid) = msg.add {
                    if t.admin.contains(&uid) {
                        self.send_message(&id, "!!! User is admin already");
                        return Box::new(fut::err(ServiceError::BadRequest));
                    }
                    let _ = write!(&mut query, "array_append(admin, {})", uid);
                }

                if let Some(uid) = msg.remove {
                    if !t.admin.contains(&uid) {
                        self.send_message(&id, "!!! User is not admin");
                        return Box::new(fut::err(ServiceError::BadRequest));
                    }
                    let _ = write!(&mut query, "array_remove(admin, {})", uid);
                }

                if query.ends_with("=") {
                    self.send_message(&id, "!!! Empty request");
                    return Box::new(fut::err(ServiceError::BadRequest));
                } else {
                    query.push_str(&format!(" WHERE id={}", tid));
                }

                let f = query_talk(self.db.as_mut().unwrap(), &query)
                    .into_actor(self)
                    .and_then(move |t, act, _| {
                        let s = serde_json::to_string(&t)
                            .unwrap_or("!!! Stringify Error.But admin query success".to_owned());
                        act.talks.insert(t.id, t);
                        act.send_message(&id, &s);
                        fut::ok(())
                    });
                Box::new(f)
            }
            None => {
                self.send_message(&id, "!!! Bad Request");
                Box::new(fut::err(ServiceError::BadRequest))
            }
        }
    }
}


#[derive(Message)]
pub struct Delete {
    pub session_id: u32,
    pub talk_id: u32,
}


impl Handler<Delete> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Delete, _: &mut Context<Self>) {
        if let Some(_) = self.talks.get(&msg.talk_id) {
            //ToDo: delete talk table and messages here.
            let string = "placeholder";

            self.send_message(&msg.session_id, string);
        }
    }
}