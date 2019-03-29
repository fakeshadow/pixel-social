use actix_web::HttpResponse;
use serde::Serialize;

pub enum Response {

    Register,
    Topic,
    Post,
    Modified,
}

impl Response {
    pub fn response(&self) -> HttpResponse {
        match &self {
            Response::Register => HttpResponse::Ok().json(Message::new("Register Success")),
            Response::Post => HttpResponse::Ok().json(Message::new("Add Post Success")),
            Response::Topic => HttpResponse::Ok().json(Message::new("Add Topic Success")),
            Response::Modified => HttpResponse::Ok().json(Message::new("Modify Success"))
        }
    }
}

#[derive(Serialize)]
pub struct Message<'a> {
    message: &'a str
}

impl<'a> Message<'a> {
    fn new(msg: &'a str) -> Self {
        Message {
            message: msg
        }
    }
}
