use actix::prelude;
use rand::{self, rngs::ThreadRng, Rng};
use std::collections::{HashMap, HashSet};

use crate::model::common::{PostgresPool, RedisPool};
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

#[derive(prelude::Message)]
pub struct SelfMessage(pub String);

#[derive(prelude::Message)]
pub struct ClientMessage {
    pub id: u32,
    pub msg: String,
    pub talk_id: u32,
}

#[derive(prelude::Message)]
pub struct Connect {
    pub session_id: u32,
    pub addr: prelude::Recipient<SelfMessage>,
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

/// sessions are hash maps with session_id(user_id from jwt) as key, addr with string message as value
/// talks are all existing talks.
/// db and cache are connection pools.
pub struct ChatServer {
    sessions: HashMap<u32, prelude::Recipient<SelfMessage>>,
    talks: Vec<Talk>,
    db: PostgresPool,
    cache: RedisPool,
}

impl prelude::Actor for ChatServer {
    type Context = prelude::Context<Self>;
}

impl ChatServer {
    pub fn init(db: PostgresPool, cache: RedisPool) -> ChatServer {
        let talks = load_all_talks(&db.get().unwrap()).unwrap();
        ChatServer {
            sessions: HashMap::new(),
            talks,
            db,
            cache,
        }
    }

    fn send_message(&self, msg: ClientMessage) {
        let _ = self.talks.iter()
            .filter(|talk| talk.id == msg.talk_id)
            .next()
            .map(|talk| &talk.users)
            .unwrap_or(&vec![])
            .into_iter()
            .map(|id| {
                if let Some(addr) = self.sessions.get(&id) {
                    let _ = addr.do_send(SelfMessage(msg.msg.to_owned()));
                }
            });
    }

    fn send_talk_members(&self, session_id: u32, talk_id: u32) {
        if let Some(addr) = self.sessions.get(&session_id) {
            let conn = self.db.get().unwrap();
            let msg = get_talk_members(talk_id, &conn).unwrap();
            addr.do_send(SelfMessage(serde_json::to_string(&msg).unwrap()));
        }
    }

    fn send_talks(&self, session_id: u32) {
        if let Some(addr) = self.sessions.get(&session_id) {
            addr.do_send(SelfMessage(serde_json::to_string(&self.talks).unwrap()));
        }
    }
}

impl prelude::Handler<Connect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut prelude::Context<Self>) {
        self.sessions.insert(msg.session_id, msg.addr);
    }
}

impl prelude::Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut prelude::Context<Self>) {
        self.sessions.remove(&msg.session_id);
    }
}

impl prelude::Handler<Create> for ChatServer {
    type Result = String;

    fn handle(&mut self, msg: Create, _: &mut prelude::Context<Self>) -> Self::Result {
        let conn = self.db.get().unwrap();
        let talk = create_talk(msg, &conn).unwrap();
        let string = serde_json::to_string(&talk).unwrap();
        self.talks.push(talk);
        string
    }
}


impl prelude::Handler<Join> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut prelude::Context<Self>) {
        let conn = self.db.get().unwrap();
        join_talk(&msg, &conn);
    }
}


impl prelude::Handler<ClientMessage> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut prelude::Context<Self>) {
        self.send_message(msg);
    }
}

impl prelude::Handler<GetRoomMembers> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: GetRoomMembers, _: &mut prelude::Context<Self>) {
        self.send_talk_members(msg.session_id, msg.talk_id)
    }
}