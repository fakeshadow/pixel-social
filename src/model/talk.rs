use actix::prelude::Message;
use chrono::NaiveDateTime;

use crate::model::user::User;

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

#[derive(Serialize)]
#[serde(tag = "type", content = "content")]
pub enum SendMessage<'a> {
    PublicMessage(&'a Vec<PublicMessage>),
    PrivateMessage(&'a Vec<PrivateMessage>),
    Users(&'a [User]),
    Talks(Vec<&'a Talk>),
    Friends(&'a [u32]),
    Success(&'a str),
    Error(&'a str),
}

impl SendMessage<'_> {
    pub fn stringify(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_|SendMessage::Error("Stringify error").stringify())
    }
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
