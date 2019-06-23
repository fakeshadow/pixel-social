use std::collections::{HashMap, HashSet};

use actix::prelude::*;
use chrono::NaiveDateTime;

use crate::model::{
    actors::TalkService,
    errors::ServiceError,
};
use crate::handler::talk::*;

#[derive(Serialize, Hash, Eq, PartialEq, Debug)]
pub struct Talk {
    pub id: u32,
    pub name: String,
    pub description: String,
    #[serde(skip_serializing)]
    #[serde(default = "default_password")]
    pub secret: String,
    pub owner: u32,
    pub admin: Vec<u32>,
    pub users: Vec<u32>,
}
fn default_password() -> String {
    "1".to_string()
}

#[derive(Message)]
pub struct SessionMessage(pub String);


#[derive(Message)]
pub struct Disconnect {
    pub session_id: u32,
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

impl Handler<Disconnect> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.sessions.remove(&msg.session_id);
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

//        self.send_message(&msg.session_id, string);
    }
}

impl Handler<Delete> for TalkService {
    type Result = ();

    fn handle(&mut self, msg: Delete, _: &mut Context<Self>) {
        if let Some(talk) = self.talks.get(&msg.talk_id) {
            //ToDo: delete talk table and messages here.
            let string = "placeholder";

//            self.send_message(&msg.session_id, string);
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
//        self.send_message(&msg.session_id, string);
    }
}

