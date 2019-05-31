use actix::prelude;
use rand::{self, rngs::ThreadRng, Rng};
use std::collections::{HashMap, HashSet};

use crate::model::common::{PostgresPool, RedisPool};

#[derive(prelude::Message)]
pub struct SelfMessage(pub String);

#[derive(prelude::Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: prelude::Recipient<SelfMessage>,
}

#[derive(prelude::Message)]
pub struct Disconnect {
    pub id: usize,
}

#[derive(prelude::Message)]
pub struct ClientMessage {
    pub id: usize,
    pub msg: String,
    pub room: String,
}

#[derive(prelude::Message)]
pub struct Join {
    /// Client id
    pub id: usize,
    /// Room name
    pub name: String,
}

#[derive(prelude::Message)]
pub struct GetRoomMembers {
    pub id: usize,
    pub room_id: usize,
}

pub struct ChatServer {
    sessions: HashMap<usize, prelude::Recipient<SelfMessage>>,
    rooms: HashMap<String, HashSet<usize>>,
    rng: ThreadRng,
    db: PostgresPool,
    cache: RedisPool,
}

impl ChatServer {
    pub fn new(db: PostgresPool, cache: RedisPool) -> ChatServer {
        let conn = cache.get().unwrap();


        let mut rooms = HashMap::new();
        rooms.insert("Main".to_owned(), HashSet::new());

        ChatServer {
            sessions: HashMap::new(),
            rooms,
            rng: rand::thread_rng(),
            db,
            cache,
        }
    }
}

impl ChatServer {
    fn send_message(&self, room: &str, message: &str, skip_id: usize) {
        if let Some(sessions) = self.rooms.get(room) {
            for id in sessions {
                if *id != skip_id {
                    if let Some(addr) = self.sessions.get(id) {
                        let _ = addr.do_send(SelfMessage(message.to_owned()));
                    }
                }
            }
        }
    }

    fn send_room_members(&self, session_id: usize, room_id: usize) {
        let conn = self.db.get().unwrap();
    }
}

impl prelude::Actor for ChatServer {
    type Context = prelude::Context<Self>;
}

impl prelude::Handler<Connect> for ChatServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut prelude::Context<Self>) -> Self::Result {
        self.send_message(&"Main".to_owned(), "Someone joined", 0);

        let id = self.rng.gen::<usize>();
        self.sessions.insert(id, msg.addr);

        self.rooms.get_mut(&"Main".to_owned()).unwrap().insert(id);

        id
    }
}

impl prelude::Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut prelude::Context<Self>) {
        let mut rooms: Vec<String> = Vec::new();

        if self.sessions.remove(&msg.id).is_some() {
            // remove session from all rooms
            for (name, sessions) in &mut self.rooms {
                if sessions.remove(&msg.id) {
                    rooms.push(name.to_owned());
                }
            }
        }
        for room in rooms {
            self.send_message(&room, "Someone disconnected", 0);
        }
    }
}

impl prelude::Handler<Join> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut prelude::Context<Self>) {
        let mut rooms: Vec<String> = Vec::new();

        if self.sessions.remove(&msg.id).is_some() {
            // remove session from all rooms
            for (name, sessions) in &mut self.rooms {
                if sessions.remove(&msg.id) {
                    rooms.push(name.to_owned());
                }
            }
        }
        for room in rooms {
            self.send_message(&room, "Someone disconnected", 0);
        }
    }
}


impl prelude::Handler<ClientMessage> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut prelude::Context<Self>) {
        self.send_message(&msg.room, msg.msg.as_str(), msg.id);
    }
}

impl prelude::Handler<GetRoomMembers> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: GetRoomMembers, _: &mut prelude::Context<Self>) {
        self.send_room_members(msg.id, msg.room_id)
    }
}