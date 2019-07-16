use std::fmt::Write;
use std::collections::HashMap;

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

impl TalkService {
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

    fn get_session(&self, session_id: &u32) -> Option<Recipient<SessionMessage>> {
        match self.sessions.lock() {
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

    fn remove_talk(&self, tid: &u32) -> Result<(), ServiceError> {
        self.talks
            .write()
            .map_err(|_| ServiceError::InternalServerError)
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

#[derive(Deserialize, Message)]
pub struct GetTalkUsers {
    pub session_id: Option<u32>,
    pub talk_id: u32,
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

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        let _ = self.sessions
            .lock()
            .map_err(|_| self.send_message(&msg.session_id, "!!! Disconnect failed"))
            .map(|mut t| t.remove(&msg.session_id));
    }
}

impl Handler<GotMessages> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: GotMessages, ctx: &mut Context<Self>) {
        // ToDo: batch insert messages to database.
        //ToDo: add time stamp before inserting

        match msg.talk_id {
            Some(id) => {
                let now = Utc::now().naive_local();
                ctx.spawn(self.db
                    .as_mut()
                    .unwrap()
                    .query(self.insert_pub_msg.as_ref().unwrap(), &[&id, &msg.msg])
                    .into_future()
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
                ctx.spawn(self.db
                    .as_mut()
                    .unwrap()
                    .query(self.insert_prv_msg.as_ref().unwrap(),
                           &[&msg.session_id.unwrap(), &id, &msg.msg])
                    .into_future()
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

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) {
        let _ = self.sessions
            .lock()
            .map_err(|_| ())
            .map(|mut t| {
                let _ = msg.addr.do_send(SessionMessage("Authentication success".to_owned()));
                t.insert(msg.session_id, msg.addr);
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
        let session_id = msg.session_id.unwrap();
        let talk_id = msg.talk_id;

        if let Some(talk) = self.get_talk(&talk_id) {
            if talk.users.contains(&session_id) {
                self.send_message(&session_id, "Already joined");
                return;
            };

            let f = self.db
                .as_mut()
                .unwrap()
                // ToDo: in case sql failed.
                .query(self.join_talk.as_ref().unwrap(),
                       &[&session_id, &talk_id])
                .into_future()
                .into_actor(self)
                .then(move |r, act, _| {
                    match r {
                        Ok((row, _)) => match row {
                            Some(_) => {
                                act.insert_user(talk_id, session_id);
                                act.send_message(&session_id, "!! Joined");
                            }
                            None => {
                                act.send_message(&session_id, "!!! Joined failed");
                            }
                        },
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
        let session_id = msg.session_id.unwrap();

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
        let id = msg.session_id.unwrap();

        let t = match self.get_talk(&tid) {
            Some(t) => t,
            None => {
                self.send_message(&id, "!!! Talk not found");
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
            return;
        }

        query.push_str(&format!(" WHERE id = {}", tid));

        let f = query_one_simple::<Talk>(self.db.as_mut().unwrap(), &query)
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
        if let Some(g) = self.talks.read().ok() {
            if !g.contains_key(&msg.talk_id) {
                return;
            }
            let session_id = msg.session_id.unwrap();

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