use actix_web::HttpResponse;
use serde::Serialize;

pub enum Response<T> {
    SendData(T),

    Register(T),
    Topic(T),
    Post(T),
    ToError(T),
    Modified(T),
}

impl<T> Response<T>
    where T: Serialize {
    pub fn response(&self) -> HttpResponse {
        match &self {
            Response::SendData(t) => HttpResponse::Ok().json(t),
            Response::Register(_) => HttpResponse::Ok().json(Message::new("Register Success")),
            Response::Post(_) => HttpResponse::Ok().json(Message::new("Add Post Success")),
            Response::Topic(_) => HttpResponse::Ok().json(Message::new("Add Topic Success")),
            Response::ToError(_) => HttpResponse::BadRequest().finish(),
            Response::Modified(_) => HttpResponse::Ok().json(Message::new("Modify Success"))
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
