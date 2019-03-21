use actix_web::HttpResponse;
use serde::Serialize;

pub enum Response<T> {
    SendData(T),

    Register(T),
    Topic(T),
    Post(T),
    ToError(T)
}

impl<T> Response<T>
    where T: Serialize {
    pub fn response(&self) -> HttpResponse {
        match &self {
            Response::SendData(t) => HttpResponse::Ok().json(t),
            Response::Register(_) => HttpResponse::Ok().json(GeneralResponse::new("Register Success")),
            Response::Post(_) => HttpResponse::Ok().json(GeneralResponse::new("Add Post Success")),
            Response::Topic(_) => HttpResponse::Ok().json(GeneralResponse::new("Add Topic Success")),
            Response::ToError(_) => HttpResponse::BadRequest().finish(),
        }
    }
}

#[derive(Serialize)]
pub struct GeneralResponse<'a> {
    message: &'a str
}

impl<'a> GeneralResponse<'a> {
    fn new(msg: &'a str) -> Self {
        GeneralResponse {
            message: msg
        }
    }
}
