use actix::prelude::Message;

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

#[derive(Message)]
pub struct SessionMessage(pub String);
