use actix::prelude::Message;

#[derive(Serialize, Hash, Eq, PartialEq, Debug)]
pub struct Talk {
    pub id: u32,
    pub name: String,
    pub description: String,
    #[serde(skip_serializing)]
    pub secret: String,
    pub owner: u32,
    pub admin: Vec<u32>,
    pub users: Vec<u32>,
}

#[derive(Message)]
pub struct SessionMessage(pub String);

