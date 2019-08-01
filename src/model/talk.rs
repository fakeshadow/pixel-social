use actix::prelude::Message;
use chrono::NaiveDateTime;

#[derive(Clone, Serialize, Debug)]
pub struct Talk {
    pub id: u32,
    pub name: String,
    pub description: String,
    #[serde(skip_serializing)]
    pub secret: String,
    pub privacy: u32,
    pub owner: u32,
    pub admin: Vec<u32>,
    pub users: Vec<u32>,
}

pub struct Relation {
    pub friends: Vec<u32>,
}

#[derive(Serialize)]
pub struct PublicMessage {
    pub talk_id: u32,
    pub time: NaiveDateTime,
    pub text: String,
}

#[derive(Serialize)]
pub struct PrivateMessage {
    pub user_id: u32,
    pub time: NaiveDateTime,
    pub text: String,
}

#[derive(Message)]
pub struct SessionMessage(pub String);
