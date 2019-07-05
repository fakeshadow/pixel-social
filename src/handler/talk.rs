use std::fmt::Write;

use actix::prelude::{
    ActorFuture,
    AsyncContext,
    Context,
    fut,
    Handler,
    Message,
    Recipient,
    Stream,
    WrapFuture,
};
use chrono::{NaiveDateTime, Utc};

use crate::model::{
    actors::TalkService,
    errors::ServiceError,
    talk::{Talk, SessionMessage},
};
use crate::handler::{
    db::{query_single_row, simple_query, query_one_simple},
    cache::get_users,
};
use std::collections::HashMap;

impl TalkService {
    fn send_message_many(&mut self, id: u32, msg: &str) {
        let _ = self.global
            .lock()
            .map_err(|_| ())
            .map(|t| {
                if let Some(talk) = t.talks.get(&id) {
                    talk.users
                        .iter()
                        .for_each(|uid| {
                            if let Some(addr) = t.sessions.get(uid) {
                                let _ = addr.do_send(SessionMessage(msg.to_owned()));
                            }
                        });
                };
            });
    }

    fn send_message(&self, session_id: &u32, msg: &str) {
        let _ = self.global
            .lock()
            .map_err(|_| ())
            .map(|t| {
                if let Some(addr) = t.sessions.get(&session_id) {
                    let _ = addr.do_send(SessionMessage(msg.to_owned()));
                };
            });
    }

    fn get_talks(&self) -> Option<HashMap<u32, Talk>> {
        match self.global.lock() {
            Ok(t) => Some(t.talks.clone()),
            Err(_) => None
        }
    }

    fn get_talk(&self, talk_id: &u32) -> Option<Talk> {
        match self.global.lock() {
            Ok(t) => t.talks.get(talk_id).map(|t| t.clone()),
            Err(_) => None
        }
    }

    fn get_session(&self, session_id: &u32) -> Option<Recipient<SessionMessage>> {
        match self.global.lock() {
            Ok(t) => t.sessions
                .get(session_id)
                .map(|addr| addr.clone()),
            Err(_) => None
        }
    }

    fn insert_talk(&self, talk: Talk) {
        let _ = self.global
            .lock()
            .map_err(|_| ())
            .map(|mut t| {
                t.talks.insert(talk.id, talk);
            });
    }

    fn remove_talk(&self, tid: &u32) -> Result<(), ServiceError> {
        self.global
            .lock()
            .map_err(|_| ServiceError::InternalServerError)
            .map(|mut t| {
                t.talks.remove(tid);
            })
    }

    fn insert_user(&self, talk_id: u32, session_id: u32) {
        let _ = self.global
            .lock()
            .map_err(|_| ())
            .map(|mut t| {
                t.talks.get_mut(&talk_id).unwrap().users.push(session_id);
            });
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

#[derive(Deserialize, Message, Clone)]
pub struct Create {
    pub session_id: u32,
    pub name: String,
    pub description: String,
    pub owner: u32,
}

#[derive(Deserialize, Message)]
pub struct Delete {
    pub session_id: u32,
    pub talk_id: u32,
}

#[derive(Deserialize, Message)]
pub struct Join {
    pub session_id: u32,
    pub talk_id: u32,
}

#[derive(Deserialize, Message)]
pub struct RemoveUser {
    pub session_id: u32,
    user_id: u32,
    talk_id: u32,
}

//impl Message for RemoveUser {
//    type Result = Result<(), ServiceError>;
//}

#[derive(Message, Deserialize)]
pub struct GetTalks {
    pub session_id: u32,
    pub talk_id: u32,
}

// pass Some(talk_id) in json for public message, pass None for private message
#[derive(Deserialize, Message)]
pub struct ClientMessage {
    pub msg: String,
    pub talk_id: Option<u32>,
    pub session_id: u32,
}

#[derive(Deserialize, Message)]
pub struct GetTalkUsers {
    pub session_id: u32,
    pub talk_id: u32,
}

// pass talk id for talk public messages. pass none for private history message.
#[derive(Deserialize, Message)]
pub struct GetHistory {
    pub time: String,
    pub talk_id: Option<u32>,
    pub session_id: u32,
}

#[derive(Deserialize, Message)]
pub struct Admin {
    pub add: Option<u32>,
    pub remove: Option<u32>,
    pub talk_id: u32,
    pub session_id: u32,
}

impl Handler<Disconnect> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        let _ = self.global
            .lock()
            .map_err(|_| self.send_message(&msg.session_id, "!!! Disconnect failed"))
            .map(|mut t| t.sessions.remove(&msg.session_id));
    }
}

impl Handler<ClientMessage> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, ctx: &mut Context<Self>) {
        // ToDo: batch insert messages to database.
        match msg.talk_id {
            Some(id) => ctx.spawn(
                self.db
                    .as_mut()
                    .unwrap()
                    .query(self.insert_pub_msg.as_ref().unwrap(), &[&id, &msg.msg])
                    .into_future()
                    .into_actor(self)
                    // ToDo: handle error.
                    .map_err(|_, _, _| ())
                    .and_then(move |(row, _), act, _| {
                        if let Some(_) = row {
                            act.send_message_many(id, &msg.msg);
                        }
                        fut::ok(())
                    })),
            // ToDo: add private message insert statement
            None => ctx.spawn(
                self.db
                    .as_mut()
                    .unwrap()
                    .query(self.insert_pub_msg.as_ref().unwrap(), &[&1, &msg.msg])
                    .into_future()
                    .into_actor(self)
                    .map_err(|_, _, _| ())
                    .and_then(move |(row, _), act, _| {
                        if let Some(_) = row {
                            act.send_message(&msg.session_id, &msg.msg);
                        }
                        fut::ok(())
                    }))
        };
    }
}

impl Handler<Connect> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) {
        let _ = self.global
            .lock()
            .map_err(|_| ())
            .map(|mut t| {
                let _ = msg.addr.do_send(SessionMessage("Authentication success".to_owned()));
                t.sessions.insert(msg.session_id, msg.addr);
            });
    }
}

impl Handler<Create> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Create, ctx: &mut Context<Self>) {
        let query = "SELECT Max(id) FROM talks";

        let f =
            query_single_row::<u32>(self.db.as_mut().unwrap(), query, 0)
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

                    query_one_simple::<Talk>(act.db.as_mut().unwrap(), &query)
                        .into_actor(act)
                        // ToDo: handle error.
                        .map_err(|_, _, _| ())
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

impl Handler<Join> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Join, ctx: &mut Context<Self>) {
        let session_id = msg.session_id;
        let talk_id = msg.talk_id;

        match self.global.lock() {
            Ok(t) => match t.talks.get(&talk_id) {
                Some(talk) => {
                    if talk.users.contains(&session_id) {
                        self.send_message(&session_id, "Already joined");
                    };

                    ctx.spawn(self.db
                        .as_mut()
                        .unwrap()
                        // ToDo: in case sql failed.
                        .query(self.join_talk.as_ref().unwrap(),
                               &[&session_id, &talk_id])
                        .into_future()
                        .into_actor(self)
                        .map_err(move |(e, _), act, _| {
                            act.send_message(&session_id, e.to_string().as_str());
                        })
                        .and_then(move |row, act, _| match row.0 {
                            Some(_) => {
                                act.insert_user(talk_id, session_id);
                                act.send_message(&session_id, "!! Joined");
                                fut::ok(())
                            }
                            None => {
                                act.send_message(&session_id, "!!! Joined failed");
                                fut::ok(())
                            }
                        }));
                }
                None => self.send_message(&session_id, "!!! Talk not found")
            }
            Err(_) => self.send_message(&session_id, "!!! Global arc lock failure")
        }
    }
}

impl Handler<GetTalks> for TalkService {
    type Result = ();
    fn handle(&mut self, msg: GetTalks, _: &mut Context<Self>) {
        match self.get_talks() {
            Some(t) => {
                let talks = match msg.session_id {
                    0 => t.iter().map(|(_, t)| t).collect(),
                    _ => t.get(&msg.talk_id).map(|t| vec![t]).unwrap_or(vec![])
                };

                let string = serde_json::to_string(&talks).unwrap_or("!!! Stringify error".to_owned());
                self.send_message(&msg.session_id, &string);
            }
            None => self.send_message(&msg.session_id, "!!! Talk not found")
        }
    }
}

impl Handler<GetHistory> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: GetHistory, ctx: &mut Context<Self>) {
        let session_id = msg.session_id;

        if let Some(addr) = self.get_session(&session_id) {
            let time = NaiveDateTime::parse_from_str(&msg.time, "%Y-%m-%d %H:%M:%S%.f")
                .unwrap_or(Utc::now().naive_local());

            let msgs = Vec::with_capacity(20);

            match msg.talk_id {
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
                        .then(move |r: Result<Vec<HistoryMessage>, ServiceError>, _, _| {
                            match r {
                                Ok(h) => {
                                    let s = serde_json::to_string(&h).unwrap_or("!!! Stringify Error".to_owned());
                                    let _ = addr.do_send(SessionMessage(s));
                                }
                                Err(_) => {
                                    let _ = addr.do_send(SessionMessage("!!! Database error".to_owned()));
                                }
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

impl Handler<GetTalkUsers> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: GetTalkUsers, ctx: &mut Context<Self>) {
        let session_id = msg.session_id;

        if let Some(addr) = self.get_session(&session_id) {
            if let Some(talk) = self.get_talk(&msg.talk_id) {
                let f = get_users(self.cache.as_ref().unwrap().clone(), talk.users)
                    .into_actor(self)
                    .then(move |r, _, _| {
                        match r {
                            Ok(u) => {
                                let string = serde_json::to_string(&u)
                                    .unwrap_or("failed to serialize users".to_owned());
                                let _ = addr.do_send(SessionMessage(string));
                            }
                            Err(_) => {
                                let _ = addr.do_send(SessionMessage("!!! Database error".to_owned()));
                            }
                        };
                        fut::ok(())
                    });
                ctx.spawn(f);
            }
        }
    }
}

impl Handler<RemoveUser> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: RemoveUser, ctx: &mut Context<Self>) {
        let id = msg.session_id;
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
                let is_admin = talk.admin.contains(&id);
                let is_owner = talk.owner == id;

                let query = if is_owner && other_is_admin {
                    format!("UPDATE talks SET admin=array_remove(admin, {}), users=array_remove(users, {})
                WHERE id={} AND owner={}", uid, uid, tid, id)
                } else if (is_admin || is_owner) & &!other_is_admin & &!other_is_owner {
                    format!("UPDATE talks SET users=array_remove(users, {})
                WHERE id={}", uid, tid)
                } else {
                    let _ = addr.do_send(SessionMessage("!!! Unauthorized".to_owned()));
                    return;
                };

                let f = query_one_simple::<Talk>(self.db.as_mut().unwrap(), &query)
                    .into_actor(self)
                    .then(move |r, act, _| {
                        match r {
                            Ok(t) => {
                                let s = serde_json::to_string(&t).unwrap_or("!!! Stringify Error.But user removal success".to_owned());
                                act.insert_talk(t);
                                act.send_message_many(tid, &s);
                            }
                            Err(_) => {
                                let _ = addr.do_send(SessionMessage("!!! Database error".to_owned()));
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
        let id = msg.session_id;


        let t = match self.global.lock() {
            Ok(g) => match g.talks.get(&tid) {
                Some(t) => t.clone(),
                None => {
                    self.send_message(&id, "!!! Talk not found");
                    return;
                }
            }
            Err(_) => {
                self.send_message(&id, "!!! Global lock failure");
                return;
            }
        };

        let mut query = "UPDATE talks SET admin=".to_owned();

        if let Some(uid) = msg.add {
            if t.admin.contains(&uid) {
                self.send_message(&id, "!!! User is admin already");
                return;
            }
            let _ = write!(&mut query, "array_append(admin, {})", uid);
        }

        if let Some(uid) = msg.remove {
            if !t.admin.contains(&uid) {
                self.send_message(&id, "!!! User is not admin");
                return;
            }
            let _ = write!(&mut query, "array_remove(admin, {})", uid);
        }

        if query.ends_with("=") {
            self.send_message(&id, "!!! Empty request");
            return;
        } else {
            query.push_str(&format!(" WHERE id={}", tid));
        }

        let f = query_one_simple::<Talk>(self.db.as_mut().unwrap(), &query)
            .into_actor(self)
            .map_err(move |_, act, _| {
                act.send_message(&id, "!!! Database Error")
            })
            .and_then(move |t, act, _| {
                let s = serde_json::to_string(&t)
                    .unwrap_or("!!! Stringify Error.But admin query success".to_owned());
                act.insert_talk(t);
                act.send_message(&id, &s);
                fut::ok(())
            });
        ctx.spawn(f);
    }
}

impl Handler<Delete> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Delete, ctx: &mut Context<Self>) {
        if let Some(g) = self.global.lock().ok() {
            if !g.talks.contains_key(&msg.talk_id) {
                return;
            }
            let session_id = msg.session_id;

            //ToDo: delete talk table and messages here.
            let query = format!("
                        DELETE FROM talks
                        WHERE id = {}", msg.talk_id);

            let f = simple_query(self.db.as_mut().unwrap(), &query)
                .into_actor(self)
                .map_err(move |_, act, _| {
                    act.send_message(&session_id, "!!! Database Error")
                })
                .and_then(move |r, act, _| {
                    let string = match r {
                        Some(_) => if act.remove_talk(&msg.talk_id).is_ok() {
                            "!! Delete talk success"
                        } else {
                            "!!! Talk not found in hash map. But delete is success."
                        },
                        None => "!!! Failed to delete talk"
                    };
                    act.send_message(&session_id, string);
                    fut::ok(())
                });

            ctx.spawn(f);
        }
    }
}