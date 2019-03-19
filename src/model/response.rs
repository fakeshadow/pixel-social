use actix_web::HttpResponse;
use serde::Serialize;

pub enum Response<T> {
    LoginSuccess(T),
    GetUserSuccess(T),
    RegisterSuccess(T),
    PostSuccess(T),
    ToError(T)
}

impl<T> Response<T>
    where T: Serialize {
    pub fn response(&self) -> HttpResponse {
        match &self {
            Response::LoginSuccess(t) => HttpResponse::Ok().json(t),
            Response::GetUserSuccess(r) => HttpResponse::Ok().json(r),
            Response::RegisterSuccess(_) => HttpResponse::Ok().json(GeneralResponse::new("Register Success")),
            Response::PostSuccess(_) => HttpResponse::Ok().json(GeneralResponse::new("Add Post Success")),

            Response::ToError(_) => HttpResponse::BadRequest().finish(),

            _ => HttpResponse::Ok().finish()
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
