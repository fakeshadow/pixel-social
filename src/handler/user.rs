use std::fmt::Write;

use actix::prelude::{
    ActorFuture,
    fut::{err, Either},
    Future,
    Handler,
    Message,
    ResponseFuture,
    ResponseActFuture,
    WrapFuture,
};

use crate::model::{
    actors::DatabaseService,
    common::GlobalVars,
    errors::ResError,
    user::{AuthRequest, AuthResponse, User, UpdateRequest},
};
use crate::handler::db;
use crate::util::hash;

pub struct GetUsers(pub Vec<u32>);

pub struct UpdateUser(pub UpdateRequest);

pub struct Register(pub AuthRequest, pub GlobalVars);

pub struct Login(pub AuthRequest);


impl Message for GetUsers {
    type Result = Result<Vec<User>, ResError>;
}

impl Message for Register {
    type Result = Result<User, ResError>;
}

impl Message for UpdateUser {
    type Result = Result<User, ResError>;
}

impl Message for Login {
    type Result = Result<AuthResponse, ResError>;
}

impl Handler<GetUsers> for DatabaseService {
    type Result = ResponseFuture<Vec<User>, ResError>;

    fn handle(&mut self, msg: GetUsers, _: &mut Self::Context) -> Self::Result {
        let mut ids = msg.0;
        ids.sort();
        ids.dedup();

        Box::new(Self::query_multi_limit(
            self.db.as_mut().unwrap(),
            self.users_by_id.as_ref().unwrap(),
            &[&ids],
            self.error_reprot.as_ref().map(|e| e.clone()),
        ))
    }
}

impl Handler<Register> for DatabaseService {
    type Result = ResponseActFuture<Self, User, ResError>;

    fn handle(&mut self, msg: Register, _: &mut Self::Context) -> Self::Result {
        let Register(req, global) = msg;
        let query = format!(
            "SELECT username, email FROM users
             WHERE username='{}' OR email='{}'", req.username, req.email.as_ref().unwrap());

        let f = db::simple_query(self.db.as_mut().unwrap(), &query)
            .into_actor(self)
            .and_then(move |m, act, _| {
                if let Some(e) = db::unique_username_email_check(&m, &req).err() {
                    return Either::A(err(e));
                }
                let hash = match hash::hash_password(&req.password) {
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
                Either::B(Self::query_one(
                    act.db.as_mut().unwrap(),
                    act.insert_user.as_ref().unwrap(),
                    &[&u.id,
                        &u.username,
                        &u.email,
                        &u.hashed_password,
                        &u.avatar_url,
                        &u.signature],
                    act.error_reprot.as_ref().map(|e| e.clone()))
                    .into_actor(act))
            });

        Box::new(f)
    }
}

impl Handler<Login> for DatabaseService {
    type Result = ResponseFuture<AuthResponse, ResError>;

    fn handle(&mut self, msg: Login, _: &mut Self::Context) -> Self::Result {
        let req = msg.0;
        let query = format!("SELECT * FROM users WHERE username='{}'", &req.username);

        Box::new(db::simple_query(self.db.as_mut().unwrap(), &query)
            .and_then(move |msg| db::auth_response_from_msg(&msg, &req.password)))
    }
}

impl Handler<UpdateUser> for DatabaseService {
    type Result = ResponseFuture<User, ResError>;

    fn handle(&mut self, msg: UpdateUser, _: &mut Self::Context) -> Self::Result {
        let u = msg.0;

        let mut query = String::new();
        query.push_str("UPDATE users SET");

        if let Some(s) = u.username.as_ref() {
            let _ = write!(&mut query, " username = '{}',", s);
        }
        if let Some(s) = u.avatar_url.as_ref() {
            let _ = write!(&mut query, " avatar_url = '{}',", s);
        }
        if let Some(s) = u.signature.as_ref() {
            let _ = write!(&mut query, " signature = '{}',", s);
        }
        if let Some(s) = u.show_email.as_ref() {
            let _ = write!(&mut query, " show_email = {},", s);
        }
        if let Some(s) = u.privilege.as_ref() {
            let _ = write!(&mut query, " privilege = {},", s);
        }

        if query.ends_with(",") {
            let _ = write!(&mut query, " updated_at = DEFAULT WHERE id = {} RETURNING *", u.id.unwrap());
        } else {
            return Box::new(futures::future::err(ResError::BadRequest));
        }

        Box::new(Self::query_one_simple(
            self.db.as_mut().unwrap(),
            query.as_str(),
            self.error_reprot.as_ref().map(|e| e.clone()),
        ))
    }
}