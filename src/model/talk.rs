use std::collections::{HashMap, HashSet};

use actix::prelude::*;
use chrono::NaiveDateTime;

use crate::model::{
    actors::TalkService,
    errors::ServiceError,
};
use crate::handler::talk::*;
use crate::handler::db::{create_talk, simple_query, get_single_row};

#[derive(Serialize, Hash, Eq, PartialEq, Debug)]
pub struct Talk {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub owner: u32,
    pub admin: Vec<u32>,
    pub users: Vec<u32>,
}

#[derive(Message)]
pub struct SessionMessage(pub String);

/// pass talk_id in json for public message, pass none for private message
#[derive(Message, Deserialize)]
pub struct ClientMessage {
    pub msg: String,
    pub talk_id: Option<u32>,
    pub session_id: u32,
}

#[derive(Serialize)]
pub struct HistoryMessage {
    pub date: NaiveDateTime,
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

#[derive(Message)]
pub struct Join {
    pub session_id: u32,
    pub talk_id: u32,
}

#[derive(Message)]
pub struct Delete {
    pub session_id: u32,
    pub talk_id: u32,
}

#[derive(Message, Deserialize)]
pub struct Remove {
    pub session_id: u32,
    pub user_id: u32,
    pub talk_id: u32,
}

#[derive(Message, Deserialize)]
pub struct Admin {
    pub add: Option<u32>,
    pub remove: Option<u32>,
    pub talk_id: u32,
    pub session_id: u32,
}

#[derive(Message)]
pub struct GetRoomMembers {
    pub session_id: u32,
    pub talk_id: u32,
}

#[derive(Message)]
pub struct GetTalks {
    pub session_id: u32,
    pub talk_id: u32,
}

/// pass talk id for talk public messages. pass none for private history message.
#[derive(Message, Deserialize)]
pub struct GetHistory {
    pub time: String,
    pub talk_id: Option<u32>,
    pub session_id: u32,
}

impl Handler<Connect> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) {
        self.sessions.insert(msg.session_id, msg.addr);
    }
}

impl Handler<Disconnect> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.sessions.remove(&msg.session_id);
    }
}

pub struct GetLastTalkId;

#[derive(Deserialize, Clone)]
pub struct Create {
    pub talk_id: Option<u32>,
    pub name: String,
    pub description: String,
    pub owner: u32,
}

impl Message for GetLastTalkId {
    type Result = Result<u32, ServiceError>;
}

impl Message for Create {
    type Result = Result<String, ServiceError>;
}

impl Handler<GetLastTalkId> for TalkService {
    type Result = ResponseFuture<u32, ServiceError>;

    fn handle(&mut self, _: GetLastTalkId, _: &mut Context<Self>) -> Self::Result {
        let query = "SELECT id FROM talks ORDER BY id DESC LIMIT 1";
        Box::new(get_single_row::<u32>(self.db.as_mut().unwrap(), query))
    }
}

impl Handler<Create> for TalkService {
    type Result = ResponseFuture<String, ServiceError>;

    fn handle(&mut self, msg: Create, ctx: &mut Context<Self>) -> Self::Result {
        let id = msg.talk_id.unwrap();
        let query1 =
            format!("INSERT INTO talks
                    (id, name, description, owner, admin, users)
                    VALUES ({}, '{}', '{}', {}, ARRAY {}, ARRAY {})",
                    id,
                    msg.name,
                    msg.description,
                    msg.owner,
                    id,
                    id);
        let query2 =
            format!("CREATE TABLE talk{}
                    (date TIMESTAMP NOT NULL PRIMARY KEY DEFAULT CURRENT_TIMESTAMP,message VARCHAR(512))",
                    id);
        let f = create_talk(self.db.as_mut().unwrap(), &query1, &query2)
            .and_then(|(_, t)| {
                let s = serde_json::to_string(&t)?;
                Ok(s)
            });
        Box::new(f)
    }
}

impl Handler<Join> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) {
        if let Some(talk) = self.talks.get(&msg.talk_id) {
            if talk.users.contains(&msg.session_id) {
                self.send_message(&msg.session_id, "Already joined")
            }
            //ToDo: push user id to database here.
            let string = "placeholder";
            self.send_message(&msg.session_id, string);
        }
    }
}

impl Handler<Remove> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Remove, _: &mut Context<Self>) {
        let string = match self.talks.get_mut(&msg.talk_id) {
            Some(talk) => {
                let (index, _) = talk.users
                    .iter()
                    .enumerate()
                    .filter(|(index, uid)| *uid == &msg.user_id)
                    .next()
                    .unwrap_or((0, &0));

                if index > 0 && talk.owner == msg.session_id {
                    //ToDo: remove user id from database here.
                    ""
                } else {
                    "!!! Wrong user id"
                }
            }
            None => "!!! Wrong talk id"
        };

        self.send_message(&msg.session_id, string);
    }
}

impl Handler<Delete> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Delete, _: &mut Context<Self>) {
        if let Some(talk) = self.talks.get(&msg.talk_id) {
            //ToDo: delete talk table and messages here.
            let string = "placeholder";

            self.send_message(&msg.session_id, string);
        }
    }
}

impl Handler<Admin> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Admin, _: &mut Context<Self>) {
        let string = match self.talks.get_mut(&msg.talk_id) {
            Some(talk) => {
                let (typ, id, can_update) = match msg.add {
                    Some(id) => ("add", id, !talk.admin.contains(&id)),
                    None => {
                        let id = msg.remove.unwrap_or(0);
                        ("remove", id, talk.admin.contains(&id))
                    }
                };
                if &talk.owner == &msg.session_id && can_update {
                    match typ {
                        //ToDo: add or remove admin to database here.
                        "add" => "placeholder",
                        _ => "placeholder"
                    }
                } else {
                    "!!! Parsing failed"
                }
            }
            None => "!!! Wrong talk"
        };
        self.send_message(&msg.session_id, string);
    }
}

impl Handler<ClientMessage> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) {
        // ToDo: batch insert messages to database.
        match msg.talk_id {
            Some(id) => {
                //ToDo: save message in db here
                self.send_message_many(id, &msg.msg);
            }
            None => {
                self.send_message(&msg.session_id, &msg.msg);
            }
        }
    }
}

impl Handler<GetHistory> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: GetHistory, _: &mut Context<Self>) {

        //ToDo: get history message from database here.

        let table = match msg.talk_id {
            Some(id) => "talk",
            None => "private"
        };
        let time = NaiveDateTime::parse_from_str(&msg.time, "%Y-%m-%d %H:%M:%S%.f");
        //ToDo: get history message from database here.

        let string = "placeholder";
        self.send_message(&msg.session_id, string);
    }
}

impl Handler<GetRoomMembers> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: GetRoomMembers, _: &mut Context<Self>) {
        self.send_talk_members(msg.session_id, msg.talk_id)
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