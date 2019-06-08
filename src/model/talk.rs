use std::collections::HashMap;

use actix::prelude;
use chrono::NaiveDateTime;
use diesel::sql_types::{VarChar, Timestamp, Bool};

use crate::model::{
    errors::ServiceError,
    common::{PostgresPool, RedisPool},
};
use crate::handler::talk::*;

use crate::schema::talks;

#[derive(Queryable, Insertable, Serialize)]
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

pub struct ChatService {
    sessions: HashMap<u32, prelude::Recipient<SessionMessage>>,
    talks: Vec<Talk>,
    db: PostgresPool,
    cache: RedisPool,
}

impl prelude::Actor for ChatService {
    type Context = prelude::Context<Self>;
}

impl ChatService {
    pub fn init(db: PostgresPool, cache: RedisPool) -> ChatService {
        let conn = &db.get().unwrap_or_else(|_| panic!("Database connection failed"));
        let talks = load_all_talks(conn).unwrap_or_else(|_| panic!("Loading talks failed"));

        ChatService {
            sessions: HashMap::new(),
            talks,
            db,
            cache,
        }
    }

    fn send_message_many(&self, msg: PublicMessage) {
        let _ = self.talks.iter()
            .filter(|talk| talk.id == msg.talk_id)
            .next()
            .map(|talk| &talk.users)
            .unwrap_or(&vec![])
            .into_iter()
            .map(|id| self.send_message(id, msg.msg.to_owned()));
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

    fn send_message(&self, session_id: &u32, msg: String) {
        if let Some(addr) = self.sessions.get(&session_id) {
            let _ = addr.do_send(SessionMessage(msg));
        }
    }
}

#[derive(prelude::Message)]
pub struct SessionMessage(pub String);


#[derive(prelude::Message, Deserialize)]
pub struct PublicMessage {
    pub msg: String,
    pub talk_id: u32,
}

#[derive(prelude::Message, Deserialize)]
pub struct PrivateMessage {
    pub msg: String,
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

impl prelude::Handler<Connect> for ChatService {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut prelude::Context<Self>) {
        self.sessions.insert(msg.session_id, msg.addr);
    }
}

impl prelude::Handler<Disconnect> for ChatService {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut prelude::Context<Self>) {
        self.sessions.remove(&msg.session_id);
    }
}

impl prelude::Handler<Create> for ChatService {
    type Result = String;

    fn handle(&mut self, msg: Create, _: &mut prelude::Context<Self>) -> Self::Result {
        let result = || -> Result<String, ServiceError> {
            let conn = self.db.get()?;
            let talk = create_talk(msg, &conn)?;
            let string = serde_json::to_string(&talk)?;
            self.talks.push(talk);
            Ok(string)
        };
        result().unwrap_or("!!! Join failed.".to_owned())
    }
}

impl prelude::Handler<Join> for ChatService {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut prelude::Context<Self>) {
        for talk in self.talks.iter() {
            if talk.users.contains(&msg.session_id) {
                self.send_message(&msg.session_id, "Already joined".to_owned())
            }
        }
        let result = || -> Result<String, ServiceError> {
            let conn = self.db.get()?;
            join_talk(&msg, &conn)?;
            Ok("!!! Joined".to_owned())
        };
        let string = result().unwrap_or("!!! Join failed.".to_owned());
        self.send_message(&msg.session_id, string);
    }
}

impl prelude::Handler<PublicMessage> for ChatService {
    type Result = ();

    fn handle(&mut self, msg: PublicMessage, _: &mut prelude::Context<Self>) {
        // ToDo: batch insert messages to database.
        let _ = insert_message("talk", &msg.talk_id, &msg.msg, &self.db);
        self.send_message_many(msg);
    }
}

impl prelude::Handler<PrivateMessage> for ChatService {
    type Result = ();

    fn handle(&mut self, msg: PrivateMessage, _: &mut prelude::Context<Self>) {
        // ToDo: batch insert messages to database.
        let _ = insert_message("private", &msg.session_id, &msg.msg, &self.db);
        self.send_message(&msg.session_id, msg.msg);
    }
}

impl prelude::Handler<GetHistory> for ChatService {
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
        self.send_message(&msg.session_id, string);
    }
}

impl prelude::Handler<GetRoomMembers> for ChatService {
    type Result = ();

    fn handle(&mut self, msg: GetRoomMembers, _: &mut prelude::Context<Self>) {
        self.send_talk_members(msg.session_id, msg.talk_id)
    }
}

impl prelude::Handler<GetTalks> for ChatService {
    type Result = ();
    fn handle(&mut self, msg: GetTalks, _: &mut prelude::Context<Self>) {
        let talks = match msg.session_id {
            0 => self.talks.iter().collect(),
            _ => self.talks.iter().filter(|t| t.id == msg.talk_id).next().map(|t| vec![t]).unwrap_or(vec![])
        };
        let string = serde_json::to_string(&talks).unwrap_or("!!! Stringify error".to_owned());
        self.send_message(&msg.session_id, string);
    }
}