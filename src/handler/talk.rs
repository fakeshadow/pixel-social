use std::collections::HashMap;
use std::fmt::Write;
use futures::{Future, future::{err as ft_err, ok as ft_ok}, IntoFuture};

use actix::prelude::*;
use chrono::NaiveDateTime;

use crate::model::{
    actors::TalkService,
    errors::ServiceError,
    user::User,
    talk::{Talk, SessionMessage, Delete},
};
use crate::handler::{
    db::{create_talk, get_single_row, simple_query},
    cache::get_users,
};
use crate::handler::db::query_talk;

impl TalkService {
    fn send_message_many(&self, id: u32, msg: &str) {
        if let Some(talk) = self.talks.get(&id) {
            talk.users.iter().map(|id| self.send_message(id, msg));
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
    pub date: NaiveDateTime,
    pub message: String,
}

#[derive(Message)]
pub struct Connect {
    pub session_id: u32,
    pub addr: Recipient<SessionMessage>,
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

/// pass talk id for talk public messages. pass none for private history message.
#[derive(Deserialize)]
pub struct GetHistory {
    pub time: String,
    pub talk_id: Option<u32>,
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

impl Handler<ClientMessage> for TalkService {
    type Result = ResponseFuture<(), ServiceError>;

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) -> Self::Result {
        // ToDo: batch insert messages to database.
        match msg.talk_id {
            Some(id) => {
                let _ = self.send_message_many(id, &msg.msg);
                let query = format!("INSERT INTO talk{} (message) VALUES ({})", &id, &msg.msg);
                let f = simple_query(self.db.as_mut().unwrap(), &query).map(|_| ());
                Box::new(f)
            }
            None => {
                let _ = self.send_message(&msg.session_id, &msg.msg);
                let query = format!("INSERT INTO private{} (message) VALUES ({})", &msg.session_id, &msg.msg);
                //ToDo : if the message insert failed because table not exist then try to creat the table and insert again.
                let f = simple_query(self.db.as_mut().unwrap(), &query).map(|_| ());
                Box::new(f)
            }
        }
    }
}

impl Handler<Connect> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Connect, ctx: &mut Context<Self>) -> Self::Result {
        self.sessions.insert(msg.session_id, msg.addr);
        self.send_message(&msg.session_id, "Authentication success");
    }
}

impl Handler<Create> for TalkService {
    type Result = ResponseActFuture<Self, (), ServiceError>;

    fn handle(&mut self, msg: Create, _: &mut Context<Self>) -> Self::Result {
        let query = "SELECT id FROM talks ORDER BY id DESC LIMIT 1";

        let f =
            get_single_row::<u32>(self.db.as_mut().unwrap(), query, 0)
                .into_actor(self)
                .and_then(move |cid, act, _| {
                    //ToDo: in case query1 array failed.
                    let query1 = format!("
                    INSERT INTO talks
                    (id, name, description, owner, admin, users)
                    VALUES ({}, '{}', '{}', {}, ARRAY [{}], ARRAY [{}])", cid, msg.name, msg.description, msg.owner, cid, cid);

                    let query2 = format!("
                    CREATE TABLE talk{}
                    (date TIMESTAMP NOT NULL PRIMARY KEY DEFAULT CURRENT_TIMESTAMP,message VARCHAR(1024))", cid);

                    create_talk(act.db.as_mut().unwrap(), &query1, &query2)
                        .into_actor(act)
                        .and_then(move |(_, t), act, _| {
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

    fn handle(&mut self, msg: Join, ctx: &mut Context<Self>) -> Self::Result {
        match self.talks.get(&msg.talk_id) {
            Some(talk) => {
                if talk.users.contains(&msg.session_id) {
                    self.send_message(&msg.session_id, "Already joined");
                    return Box::new(fut::err(ServiceError::BadRequest));
                };
                // ToDo: in case sql failed.
                let query = format!("UPDATE talks SET users=array_append(users, '{}') WHERE id={}", &msg.session_id, &msg.talk_id);
                let f = simple_query(self.db.as_mut().unwrap(), &query)
                    .map(|_| ())
                    .into_actor(self)
                    .then(move |r, act, _| match r {
                        Ok(_) => {
                            act.talks.get_mut(&msg.talk_id).unwrap().users.push(msg.session_id);
                            act.send_message(&msg.session_id, "!! Joined");
                            fut::ok(())
                        }
                        Err(_) => {
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
    type Result = ResponseFuture<(), ServiceError>;

    fn handle(&mut self, msg: GetHistory, _: &mut Context<Self>) -> Self::Result {
        if let Some(addr) = self.sessions.get(&msg.session_id) {
            let table = match msg.talk_id {
                Some(id) => "talk",
                None => "private"
            };
            let time = NaiveDateTime::parse_from_str(&msg.time, "%Y-%m-%d %H:%M:%S%.f").unwrap();

            let query = format!("SELECT * FROM {}{} WHERE date <= {} ORDER BY date DESC LIMIT 20", table, msg.session_id, time);
            //ToDo: query db and get messages.
            let addr = addr.clone();

            return Box::new(ft_err(ServiceError::BadRequest));
        }

        Box::new(ft_err(ServiceError::BadRequest))
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

//pub fn add_admin(
//    id: u32,
//    talk_id: u32,
//    pool: &PostgresPool,
//) -> Result<(), ServiceError> {
//    let conn = &pool.get()?;
//    let mut ids: Vec<u32> = talks::table.find(talk_id).select(talks::admin).first::<Vec<u32>>(conn)?;
//    ids.push(id);
//    ids.sort();
//    let _ = diesel::update(talks::table.find(talk_id)).set(talks::admin.eq(ids)).execute(conn)?;
//    Ok(())
//}
//
//pub fn remove_id(
//    id: u32,
//    mut ids: Vec<u32>,
//) -> Result<Vec<u32>, ServiceError> {
//    let (index, _) = ids
//        .iter()
//        .enumerate()
//        .filter(|(i, uid)| *uid == &id)
//        .next()
//        .ok_or(ServiceError::InternalServerError)?;
//    ids.remove(index);
//    Ok(ids)
//}