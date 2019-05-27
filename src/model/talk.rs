use actix::prelude;
use rand::{self, rngs::ThreadRng, Rng};
use std::collections::{HashMap, HashSet};

/// Chat server sends this messages to session
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

pub struct ListRooms;

impl prelude::Message for ListRooms {
    type Result = Vec<String>;
}

#[derive(prelude::Message)]
pub struct Join {
    /// Client id
    pub id: usize,
    /// Room name
    pub name: String,
}

/// `ChatServer` manages chat rooms and responsible for coordinating chat
/// session. implementation is super primitive
pub struct ChatServer {
    sessions: HashMap<usize, prelude::Recipient<SelfMessage>>,
    rooms: HashMap<String, HashSet<usize>>,
    rng: ThreadRng,
}

impl Default for ChatServer {
    fn default() -> ChatServer {
        // default room
        let mut rooms = HashMap::new();
        rooms.insert("Main".to_owned(), HashSet::new());

        ChatServer {
            sessions: HashMap::new(),
            rooms,
            rng: rand::thread_rng(),
        }
    }
}

impl ChatServer {
    /// Send message to all users in the room
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

        // remove address
        if self.sessions.remove(&msg.id).is_some() {
            // remove session from all rooms
            for (name, sessions) in &mut self.rooms {
                if sessions.remove(&msg.id) {
                    rooms.push(name.to_owned());
                }
            }
        }
        // send message to other users
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

/// Handler for `ListRooms` message.
impl prelude::Handler<ListRooms> for ChatServer {
    type Result = prelude::MessageResult<ListRooms>;

    fn handle(&mut self, _: ListRooms, _: &mut prelude::Context<Self>) -> Self::Result {
        let mut rooms = Vec::new();

        for key in self.rooms.keys() {
            rooms.push(key.to_owned())
        }

        prelude::MessageResult(rooms)
    }
}

/// Join room, send disconnect message to old room
/// send join message to new room
impl prelude::Handler<Join> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut prelude::Context<Self>) {
        let Join { id, name } = msg;
        let mut rooms = Vec::new();

        // remove session from all rooms
        for (n, sessions) in &mut self.rooms {
            if sessions.remove(&id) {
                rooms.push(n.to_owned());
            }
        }
        // send message to other users
        for room in rooms {
            self.send_message(&room, "Someone disconnected", 0);
        }

        if self.rooms.get_mut(&name).is_none() {
            self.rooms.insert(name.clone(), HashSet::new());
        }
        self.send_message(&name, "Someone connected", id);
        self.rooms.get_mut(&name).unwrap().insert(id);
    }
}