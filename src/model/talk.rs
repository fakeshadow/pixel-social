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
pub struct Delete {
    pub session_id: u32,
    pub talk_id: u32,
}


#[derive(Message, Deserialize)]
pub struct Admin {
    pub add: Option<u32>,
    pub remove: Option<u32>,
    pub talk_id: u32,
    pub session_id: u32,
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

