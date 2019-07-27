use std::fmt::Write;

use actix::prelude::{
    Addr,
    ActorFuture,
    AsyncContext,
    Context,
    fut,
    Handler,
    Message,
    Stream,
    WrapFuture,
};
use chrono::{NaiveDateTime, Utc};
use hashbrown::HashMap;
use redis::cmd;

use crate::model::{
    actors::{TalkService, WsChatSession},
    errors::ResError,
    talk::{Talk, SessionMessage},
};
use crate::handler::{
    cache::get_users,
};

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

#[derive(Serialize)]
struct HistoryMessage {
    pub talk_id: u32,
    pub time: NaiveDateTime,
    pub message: String,
}

#[derive(Deserialize)]
pub struct Auth {
    pub token: String,
    pub online_status: u32,
}

#[derive(Message)]
pub struct Connect {
    pub session_id: u32,
    pub online_status: u32,
    pub addr: Addr<WsChatSession>,
}

#[derive(Message)]
pub struct Disconnect {
    pub session_id: u32,
}

#[derive(Deserialize, Message, Clone)]
pub struct Create {
    pub session_id: Option<u32>,
    pub name: String,
    pub description: String,
    pub owner: u32,
}

#[derive(Deserialize, Message)]
pub struct Delete {
    pub session_id: Option<u32>,
    pub talk_id: u32,
}

#[derive(Deserialize, Message)]
pub struct Join {
    pub session_id: Option<u32>,
    pub talk_id: u32,
}

#[derive(Deserialize, Message)]
pub struct RemoveUser {
    pub session_id: Option<u32>,
    user_id: u32,
    talk_id: u32,
}

#[derive(Message, Deserialize)]
pub struct GetRelation {
    pub session_id: Option<u32>,
}

#[derive(Message, Deserialize)]
pub struct GetUsers {
    pub session_id: Option<u32>,
    user_id: Vec<u32>,
}

#[derive(Message, Deserialize)]
pub struct GetTalks {
    pub session_id: Option<u32>,
    pub talk_id: u32,
}

// pass Some(talk_id) in json for public message, pass None for private message
#[derive(Deserialize, Message)]
pub struct GotMessages {
    pub msg: String,
    pub talk_id: Option<u32>,
    pub user_id: Option<u32>,
    pub session_id: Option<u32>,
}

#[derive(Serialize)]
pub struct ClientMessage<'a> {
    pub msg: &'a str,
    pub time: &'a NaiveDateTime,
    pub user_id: Option<&'a u32>,
    pub talk_id: Option<&'a u32>,
}

#[derive(Serialize)]
struct SendRelation<'a> {
    typ: &'a str,
    friends: Vec<u32>,
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

impl Handler<Disconnect> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, ctx: &mut Context<Self>) {
        let _ = self.sessions
            .write()
            .map_err(|_| self.send_message(&msg.session_id, "!!! Disconnect failed"))
            .map(|mut t| {
                let f = cmd("HMSET")
                    .arg(&format!("user:{}:set", msg.session_id))
                    .arg(&[("online_status", 0.to_string()), ("last_online", Utc::now().naive_local().to_string())])
                    .query_async(self.cache.as_ref().unwrap().clone())
                    .into_actor(self)
                    .map_err(|_, _, _| ())
                    .map(|(_, ()), _, _| ());
                ctx.spawn(f);

                t.remove(&msg.session_id)
            });
    }
}

impl Handler<GotMessages> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: GotMessages, ctx: &mut Context<Self>) {
        // ToDo: batch insert messages to database.

        match msg.talk_id {
            Some(id) => {
                let now = Utc::now().naive_local();
                ctx.spawn(Self::query_one(
                    self.db.as_mut().unwrap(),
                    self.insert_pub_msg.as_ref().unwrap(),
                    &[&id, &msg.msg, &now])
                    .into_actor(self)
                    .then(move |r, act, _| match r {
                        Ok(_) => {
                            let mut result = "/msg ".to_owned();
                            let vec = vec![ClientMessage {
                                msg: &msg.msg,
                                time: &now,
                                user_id: None,
                                talk_id: msg.talk_id.as_ref(),
                            }];
                            let temp = serde_json::to_string(&vec).unwrap();
                            result.push_str(&temp);
                            act.send_message_many(id, &result);
                            fut::ok(())
                        }
                        Err(_) => {
                            act.send_message(msg.session_id.as_ref().unwrap(), "!!! Database error");
                            fut::ok(())
                        }
                    }));
            }
            None => {
                let id = match msg.user_id {
                    Some(id) => id,
                    None => return self.send_message(&msg.session_id.unwrap(), "!!! No user found")
                };
                let now = Utc::now().naive_local();
                ctx.spawn(Self::query_one(
                    self.db.as_mut().unwrap(),
                    self.insert_prv_msg.as_ref().unwrap(),
                    &[&msg.session_id.unwrap(), &id, &msg.msg])
                    .into_actor(self)
                    // ToDo: handle error.
                    .map_err(|_, _, _| ())
                    .then(move |r, act, _| match r {
                        Ok(_) => {
                            let mut result = "/msg ".to_owned();
                            let vec = vec![ClientMessage {
                                msg: &msg.msg,
                                time: &now,
                                user_id: msg.user_id.as_ref(),
                                talk_id: None,
                            }];
                            let temp = serde_json::to_string(&vec).unwrap();
                            result.push_str(&temp);
                            act.send_message(&id, &result);
                            fut::ok(())
                        }
                        Err(_) => {
                            act.send_message(msg.session_id.as_ref().unwrap(), "!!! Database error");
                            fut::ok(())
                        }
                    }));
            }
        };
    }
}

impl Handler<Connect> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Connect, ctx: &mut Context<Self>) {
        let _ = self.sessions
            .write()
            .map_err(|_| ())
            .map(|mut t| {
                let f = cmd("HMSET")
                    .arg(&format!("user:{}:set", msg.session_id))
                    .arg(&[("online_status", msg.online_status.to_string())])
                    .query_async(self.cache.as_ref().unwrap().clone())
                    .into_actor(self)
                    .map_err(|_, _, _| ())
                    .map(|(_, ()), _, _| ());
                ctx.spawn(f);
                let _ = msg.addr.do_send(SessionMessage("! Authentication success".to_owned()));
                t.insert(msg.session_id, msg.addr);
            });
    }
}

impl Handler<Create> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Create, ctx: &mut Context<Self>) {
        let query = "SELECT Max(id) FROM talks";

        let f =
            self.simple_query_single_row::<u32>(query, 0)
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

                    act.simple_query_talk(query.as_str())
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
        let session_id = msg.session_id.unwrap();
        let talk_id = msg.talk_id;

        if let Some(talk) = self.get_talk(&talk_id) {
            if talk.users.contains(&session_id) {
                self.send_message(&session_id, "Already joined");
                return;
            };

            let f = Self::query_one(
                self.db.as_mut().unwrap(),
                self.join_talk.as_ref().unwrap(),
                &[&session_id, &talk_id])
                .into_actor(self)
                .then(move |r, act, _| {
                    match r {
                        Ok(_) => {
                            act.insert_user(talk_id, session_id);
                            act.send_message(&session_id, "!! Joined");
                        }
                        Err(_) => act.send_message(&session_id, "!!! Database Error")
                    };
                    fut::ok(())
                });
            ctx.spawn(f);
        }
    }
}

impl Handler<GetTalks> for TalkService {
    type Result = ();
    fn handle(&mut self, msg: GetTalks, _: &mut Context<Self>) {
        match self.get_talks() {
            Some(t) => {
                let talks = match msg.talk_id {
                    0 => t.iter().map(|(_, t)| t).collect(),
                    _ => t.get(&msg.talk_id).map(|t| vec![t]).unwrap_or(vec![])
                };

                let mut result = "/talks ".to_owned();
                let string = serde_json::to_string(&talks).unwrap_or("!!! Stringify error".to_owned());
                result.push_str(&string);

                self.send_message(&msg.session_id.unwrap(), &result);
            }
            None => self.send_message(&msg.session_id.unwrap(), "!!! Talk not found")
        }
    }
}

impl Handler<GetUsers> for TalkService {
    type Result = ();
    fn handle(&mut self, msg: GetUsers, ctx: &mut Context<Self>) {
        let session_id = msg.session_id.unwrap();
        if let Some(addr) = self.get_session(&session_id) {
            let f = get_users(self.get_conn(), &msg.user_id)
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

impl Handler<GetRelation> for TalkService {
    type Result = ();
    fn handle(&mut self, msg: GetRelation, ctx: &mut Context<Self>) {
        let f = self.db
            .as_mut()
            .unwrap()
            // ToDo: add query for relations table for user friends.
            .query(self.get_relations.as_ref().unwrap(), &[msg.session_id.as_ref().unwrap()])
            .into_future()
            .into_actor(self)
            .then(move |r, act, _| match r {
                Ok((r, _)) => {
                    if let Some(r) = r {
                        let s = serde_json::to_string(&SendRelation {
                            typ: "relation",
                            friends: r.get(1),
                        }).unwrap_or("!!! Stringify Error".to_owned());

                        let _ = act.send_message(msg.session_id.as_ref().unwrap(), &s.as_str());
                    };
                    fut::ok(())
                }
                Err((_, _)) => {
                    let _ = act.send_message(msg.session_id.as_ref().unwrap(), "!!! Database error");
                    fut::ok(())
                }
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
                            Ok::<Vec<HistoryMessage>, ResError>(msgs)
                        })
                        .into_actor(self)
                        .then(move |r: Result<Vec<HistoryMessage>, ResError>, _, _| {
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

impl Handler<RemoveUser> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: RemoveUser, ctx: &mut Context<Self>) {
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
                let is_admin = talk.admin.contains(&id);
                let is_owner = talk.owner == id;

                let query = if is_owner && other_is_admin {
                    format!("UPDATE talks SET admin=array_remove(admin, {}), users=array_remove(users, {})
                WHERE id={} AND owner={}", uid, uid, tid, id)
                } else if (is_admin || is_owner) && !other_is_admin && !other_is_owner {
                    format!("UPDATE talks SET users=array_remove(users, {})
                WHERE id={}", uid, tid)
                } else {
                    let _ = addr.do_send(SessionMessage("!!! Unauthorized".to_owned()));
                    return;
                };

                let f = self.simple_query_talk(query.as_str())
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

        let f = self.simple_query_talk(query.as_str())
            .into_actor(self)
            .then(move |r, act, _| {
                match r {
                    Ok(t) => {
                        let s = serde_json::to_string(&t)
                            .unwrap_or("!!! Stringify Error.But admin query success".to_owned());
                        act.insert_talk(t);
                        act.send_message(&id, &s);
                    }
                    Err(_) => act.send_message(&id, "!!! Database Error")
                };
                fut::ok(())
            });
        ctx.spawn(f);
    }
}

impl Handler<Delete> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Delete, ctx: &mut Context<Self>) {
        if let Some(_) = self.get_talk(&msg.talk_id) {
            let session_id = msg.session_id.unwrap();

            //ToDo: delete talk table and messages here.
            let query = format!("
                        DELETE FROM talks
                        WHERE id = {}", msg.talk_id);

            let f = self.simple_query_talk(query.as_str())
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