use std::collections::{HashMap, HashSet};

use actix::prelude;
use chrono::NaiveDateTime;
use diesel::sql_types::{VarChar, Timestamp, Bool};

use crate::model::{
    errors::ServiceError,
    common::{PostgresPool, RedisPool},
};
use crate::handler::talk::*;

use crate::schema::talks;

#[derive(Queryable, Insertable, Serialize, Hash, Eq, PartialEq)]
#[table_name = "talks"]
pub struct Talk {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub owner: u32,
    pub admin: Vec<u32>,
    pub users: Vec<u32>,
}

impl Talk {
    pub fn new(id: u32, msg: Create) -> Self {
        Talk {
            id,
            name: msg.name,
            description: msg.description,
            owner: msg.owner,
            admin: vec![],
            users: vec![],
        }
    }
}

pub struct TalkService {
    sessions: HashMap<u32, prelude::Recipient<SessionMessage>>,
    talks: HashMap<u32, Talk>,
    db: PostgresPool,
    cache: RedisPool,
}

impl prelude::Actor for TalkService {
    type Context = prelude::Context<Self>;
}

impl TalkService {
    pub fn init(db: PostgresPool, cache: RedisPool) -> TalkService {
        let conn = &db.get().unwrap_or_else(|_| panic!("Database connection failed"));
        let talks = load_all_talks(conn).unwrap_or_else(|_| panic!("Loading talks failed"));
        let mut hash = HashMap::new();

        for talk in talks.into_iter() {
            hash.insert(talk.id, talk);
        }

        TalkService {
            sessions: HashMap::new(),
            talks: hash,
            db,
            cache,
        }
    }

    fn send_message_many(&self, id: u32, msg: &str) {
        if let Some(talk) = self.talks.get(&id) {
            talk.users.iter().map(|id| self.send_message(id, msg));
        }
    }

    fn send_talk_members(&self, session_id: u32, talk_id: u32) {
        if let Some(addr) = self.sessions.get(&session_id) {
            let result = || -> Result<String, ServiceError> {
                let conn = self.db.get()?;
                let msg = get_talk_members(talk_id, &conn)?;
                Ok(serde_json::to_string(&msg)?)
            };
            let msg = result().unwrap_or("!!! Failed to get talk members".to_owned());
            addr.do_send(SessionMessage(msg));
        }
    }

    fn send_message(&self, session_id: &u32, msg: &str) {
        if let Some(addr) = self.sessions.get(&session_id) {
            let _ = addr.do_send(SessionMessage(msg.to_owned()));
        }
    }
}

#[derive(prelude::Message)]
pub struct SessionMessage(pub String);

/// pass talk_id in json for public message, pass none for private message
#[derive(prelude::Message, Deserialize)]
pub struct ClientMessage {
    pub msg: String,
    pub talk_id: Option<u32>,
    pub session_id: u32,
}

#[derive(QueryableByName, Serialize)]
pub struct HistoryMessage {
    #[sql_type = "Timestamp"]
    pub date: NaiveDateTime,
    #[sql_type = "VarChar"]
    pub message: String,
}

#[derive(prelude::Message)]
pub struct Connect {
    pub session_id: u32,
    pub addr: prelude::Recipient<SessionMessage>,
}

#[derive(prelude::Message)]
pub struct Disconnect {
    pub session_id: u32,
}

#[derive(prelude::Message)]
#[rtype(String)]
pub struct Create {
    pub name: String,
    pub description: String,
    pub owner: u32,
}

#[derive(prelude::Message)]
pub struct Join {
    pub session_id: u32,
    pub talk_id: u32,
}

#[derive(prelude::Message)]
pub struct Delete {
    pub session_id: u32,
    pub talk_id: u32,
}

#[derive(prelude::Message, Deserialize)]
pub struct Remove {
    pub session_id: u32,
    pub user_id: u32,
    pub talk_id: u32,
}

#[derive(prelude::Message)]
pub struct GetRoomMembers {
    pub session_id: u32,
    pub talk_id: u32,
}

#[derive(prelude::Message)]
pub struct GetTalks {
    pub session_id: u32,
    pub talk_id: u32,
}

/// pass talk id for talk public messages. pass none for private history message.
#[derive(prelude::Message, Deserialize)]
pub struct GetHistory {
    pub time: String,
    pub talk_id: Option<u32>,
    pub session_id: u32,
}

impl prelude::Handler<Connect> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut prelude::Context<Self>) {
        self.sessions.insert(msg.session_id, msg.addr);
    }
}

impl prelude::Handler<Disconnect> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut prelude::Context<Self>) {
        self.sessions.remove(&msg.session_id);
    }
}

impl prelude::Handler<Create> for TalkService {
    type Result = String;

    fn handle(&mut self, msg: Create, _: &mut prelude::Context<Self>) -> Self::Result {
        let result = || -> Result<String, ServiceError> {
            let conn = self.db.get()?;
            let talk = create_talk(msg, &conn)?;
            let string = serde_json::to_string(&talk)?;
            self.talks.insert(talk.id, talk);
            Ok(string)
        };
        result().unwrap_or("!!! Join failed.".to_owned())
    }
}

impl prelude::Handler<Join> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut prelude::Context<Self>) {
        if let Some(talk) = self.talks.get(&msg.talk_id) {
            if talk.users.contains(&msg.session_id) {
                self.send_message(&msg.session_id, "Already joined")
            }

            let result = || -> Result<&str, ServiceError> {
                let conn = self.db.get()?;
                join_talk(&msg, &conn)?;
                Ok("!!! Joined")
            };
            let string = result().unwrap_or("!!! Join failed.");
            self.send_message(&msg.session_id, string);
        }
    }
}

impl prelude::Handler<Remove> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Remove, _: &mut prelude::Context<Self>) {

        let string = match self.talks.get_mut(&msg.talk_id) {
            Some(talk) => {
                let (index, _) = talk.users
                    .iter()
                    .enumerate()
                    .filter(|(index, uid)| *uid == &msg.user_id)
                    .next()
                    .unwrap_or((0, &0));

                if index > 0 && talk.owner == msg.session_id {
                    remove_talk_member(msg.user_id, msg.talk_id, &self.db)
                        .map(|_| {
                            talk.users.remove(index);
                            "Removed"
                        })
                        .unwrap_or("!!! Remove Failed")
                } else {
                    "!!! Wrong user id"
                }
            }
            None => "!!! Wrong talk id"
        };

        self.send_message(&msg.session_id, string);
    }
}

impl prelude::Handler<Delete> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Delete, _: &mut prelude::Context<Self>) {
        if let Some(talk) = self.talks.get(&msg.talk_id) {
            let result = || -> Result<(), ServiceError> {
                if &talk.owner == &msg.session_id {
                    remove_talk(&msg, &self.db.get()?)
                } else {
                    Err(ServiceError::InternalServerError)
                }
            };
            let string = result()
                .map(|_| {
                    self.talks.remove(&msg.talk_id);
                    "Deleted"
                })
                .unwrap_or("!!! Wrong talk");

            self.send_message(&msg.session_id, string);
        }
    }
}

impl prelude::Handler<ClientMessage> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut prelude::Context<Self>) {
        // ToDo: batch insert messages to database.
        match msg.talk_id {
            Some(id) => {
                let _ = insert_message("talk", &id, &msg.msg, &self.db);
                self.send_message_many(id, &msg.msg);
            }
            None => {
                let _ = insert_message("private", &msg.session_id, &msg.msg, &self.db);
                self.send_message(&msg.session_id, &msg.msg);
            }
        }
    }
}

impl prelude::Handler<GetHistory> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: GetHistory, _: &mut prelude::Context<Self>) {
        let result = || -> Result<String, ServiceError> {
            let table = match msg.talk_id {
                Some(id) => "talk",
                None => "private"
            };
            let time = NaiveDateTime::parse_from_str(&msg.time, "%Y-%m-%d %H:%M:%S%.f")?;
            let history = get_history(table, msg.talk_id.unwrap_or(msg.session_id), &time, &self.db.get()?)?;
            Ok(serde_json::to_string(&history)?)
        };

        let string = result().unwrap_or("!!! Failed to get history message".to_owned());
        self.send_message(&msg.session_id, &string);
    }
}

impl prelude::Handler<GetRoomMembers> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: GetRoomMembers, _: &mut prelude::Context<Self>) {
        self.send_talk_members(msg.session_id, msg.talk_id)
    }
}

impl prelude::Handler<GetTalks> for TalkService {
    type Result = ();
    fn handle(&mut self, msg: GetTalks, _: &mut prelude::Context<Self>) {
        let talks = match msg.session_id {
            0 => self.talks.iter().map(|(_, t)| t).collect(),
            _ => self.talks.get(&msg.talk_id).map(|t| vec![t]).unwrap_or(vec![])
        };
        let string = serde_json::to_string(&talks).unwrap_or("!!! Stringify error".to_owned());
        self.send_message(&msg.session_id, &string);
    }
}