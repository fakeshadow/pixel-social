use actix_web::{dev, FromRequest, HttpRequest};

use actix::prelude::{
    ActorFuture,
    fut::{err, Either},
    Future,
    Handler,
    Message,
    ResponseActFuture,
    ResponseFuture,
    WrapFuture,
};

use crate::{
    CacheService,
    DatabaseService,
};
use crate::model::{
    user::{User, AuthRequest, AuthResponse},
    common::GlobalVars,
    errors::ResError,
};
use crate::util::jwt::JwtPayLoad;

pub type UserJwt = JwtPayLoad;

// jwt token extractor from request
impl FromRequest for JwtPayLoad {
    type Error = ResError;
    type Future = Result<UserJwt, ResError>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        match req.headers().get("Authorization") {
            Some(token) => {
                let vec: Vec<&str> = token
                    .to_str()
                    .unwrap_or("no token")
                    .rsplitn(2, " ")
                    .collect();
                JwtPayLoad::from(vec[0])
            }
            None => Err(ResError::Unauthorized)
        }
    }
}


pub struct Register(pub AuthRequest, pub GlobalVars);

impl Message for Register {
    type Result = Result<User, ResError>;
}

impl Handler<Register> for DatabaseService {
    type Result = ResponseActFuture<Self, User, ResError>;

    fn handle(&mut self, msg: Register, _: &mut Self::Context) -> Self::Result {
        let Register(req, global) = msg;
        let query = format!(
            "SELECT username, email FROM users
             WHERE username='{}' OR email='{}'", req.username, req.email.as_ref().unwrap());

        let f = self
            .unique_username_email_check(query.as_str(), req)
            .into_actor(self)
            .and_then(move |req: AuthRequest, act, _| {
                let hash = match crate::util::hash::hash_password(&req.password) {
                    Ok(hash) => hash,
                    Err(e) => return Either::A(err(e))
                };
                let id = match global.lock() {
                    Ok(mut var) => var.next_uid(),
                    Err(_) => return Either::A(err(ResError::InternalServerError))
                };
                let u = match req.make_user(&id, &hash) {
                    Ok(u) => u,
                    Err(e) => return Either::A(err(e))
                };
                Either::B(act
                    .insert_user(&[
                        &u.id,
                        &u.username,
                        &u.email,
                        &u.hashed_password,
                        &u.avatar_url,
                        &u.signature
                    ])
                    .into_actor(act))
            });

        Box::new(f)
    }
}


pub struct Login(pub AuthRequest);

impl Message for Login {
    type Result = Result<AuthResponse, ResError>;
}

impl Handler<Login> for DatabaseService {
    type Result = ResponseFuture<AuthResponse, ResError>;

    fn handle(&mut self, msg: Login, _: &mut Self::Context) -> Self::Result {
        let req = msg.0;
        let query = format!("SELECT * FROM users WHERE username='{}'", &req.username);

        Box::new(self.generate_auth_response(query.as_str(), req.password))
    }
}


pub struct ActivateUser(pub String);

impl Message for ActivateUser {
    type Result = Result<u32, ResError>;
}

impl Handler<ActivateUser> for CacheService {
    type Result = ResponseFuture<u32, ResError>;

    fn handle(&mut self, msg: ActivateUser, _: &mut Self::Context) -> Self::Result {
        let f = self
            .get_hash_map(&msg.0)
            .and_then(|hm| Ok(hm
                .get("user_id")
                .ok_or(ResError::Unauthorized)?
                .parse::<u32>()?)
            );
        Box::new(f)
    }
}