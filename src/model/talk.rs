use actix::prelude;
use rand::{self, rngs::ThreadRng, Rng};
use std::collections::{HashMap, HashSet};

use crate::model::common::{PostgresPool, RedisPool};
use crate::handler::talk::*;

use crate::schema::talks;

#[derive(Queryable, Insertable)]
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
#[rtype(usize)]
pub struct Connect {
    pub addr: prelude::Recipient<SelfMessage>,
}

#[derive(prelude::Message)]
pub struct Disconnect {
    pub id: u32,
}

#[derive(prelude::Message)]
pub struct ClientMessage {
    pub id: u32,
    pub msg: String,
    pub room_id: u32,
}

#[derive(prelude::Message)]
pub struct Create {
    pub name: String,
    pub description: String,
    pub owner: u32,
}

#[derive(prelude::Message)]
pub struct Join {
    pub id: u32,
    pub user_id: u32,
}

#[derive(prelude::Message)]
pub struct GetRoomMembers {
    pub id: u32,
    pub room_id: u32,
}

/// sessions are hash maps with user_id as key, addr with string message as value
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
            .filter(|talk| talk.id == msg.room_id)
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

    fn send_room_members(&self, session_id: usize, room_id: u32) {
        if let Some(addr) = self.sessions.get(&room_id) {
            let conn = self.db.get().unwrap();
            let message = get_room_members(room_id as u32, &conn).unwrap();
            addr.do_send(SelfMessage(serde_json::to_string(&message).unwrap()));
        }
    }
}

impl prelude::Handler<Connect> for ChatServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut prelude::Context<Self>) -> Self::Result {
        self.send_message(ClientMessage {
            id: 0,
            msg: "Test".to_owned(),
            room_id: 1,
        });

        let id = 1usize;
        self.sessions.insert(id as u32, msg.addr);
        id
    }
}

impl prelude::Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut prelude::Context<Self>) {
//        let mut rooms: Vec<String> = Vec::new();
//
//        if self.sessions.remove(&msg.id).is_some() {
//            // remove session from all rooms
//            for (name, sessions) in &mut self.rooms {
//                if sessions.remove(&msg.id) {
//                    rooms.push(name.to_owned());
//                }
//            }
//        }
//        for room in rooms {
//            self.send_message(ClientMessage {
//                id: 0,
//                msg: "Test".to_owned(),
//                room_id: 1,
//            });
//        }
    }
}

impl prelude::Handler<Create> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Create, _: &mut prelude::Context<Self>) {
        let conn = self.db.get().unwrap();
        let talk = create_talk(msg, &conn).unwrap();
        self.talks.push(talk);
    }
}


impl prelude::Handler<Join> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut prelude::Context<Self>) {}
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
        self.send_room_members(msg.id as usize, msg.room_id)
    }
}