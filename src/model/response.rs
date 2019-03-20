use actix_web::HttpResponse;
use serde::Serialize;

pub enum Response<T> {
    GetUser(T),
    GetPost(T),
    GetTopic(T),
    Register(T),
    Login(T),
    Topic(T),
    Post(T),
    ToError(T)
}

impl<T> Response<T>
    where T: Serialize {
    pub fn response(&self) -> HttpResponse {
        match &self {
            Response::Login(t) => HttpResponse::Ok().json(t),
            Response::GetUser(r) => HttpResponse::Ok().json(r),
            Response::GetPost(r) => HttpResponse::Ok().json(r),
            Response::GetTopic(r) => HttpResponse::Ok().json(r),
            Response::Register(_) => HttpResponse::Ok().json(GeneralResponse::new("Register Success")),
            Response::Post(_) => HttpResponse::Ok().json(GeneralResponse::new("Add Post Success")),
            Response::Topic(_) => HttpResponse::Ok().json(GeneralResponse::new("Add Topic Success")),

            Response::ToError(_) => HttpResponse::BadRequest().finish(),

//            _ => HttpResponse::Ok().finish()
        }
    }
}

#[derive(Debug, Serialize)]
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
