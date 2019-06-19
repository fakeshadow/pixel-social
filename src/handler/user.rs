use std::fmt::Write;
use futures::{Future, future, IntoFuture};

use actix::prelude::*;
use chrono::NaiveDateTime;

use crate::model::{
    actors::DatabaseService,
    common::GlobalGuard,
    errors::ServiceError,
    user::{AuthRequest, AuthResponse, User, UpdateRequest},
};
use crate::handler::db::{
    get_users,
    user_from_msg,
    auth_response_from_msg,
    unique_username_email_check,
    simple_query,
};
use crate::util::{hash, jwt};


pub struct GetUsers(pub Vec<u32>);

pub struct UpdateUser(pub UpdateRequest);

pub struct PreRegister(pub AuthRequest);

pub struct Register(pub AuthRequest, pub GlobalGuard);

pub struct Login(pub AuthRequest);


impl Message for GetUsers {
    type Result = Result<Vec<User>, ServiceError>;
}

impl Message for Register {
    type Result = Result<Vec<User>, ServiceError>;
}

impl Message for UpdateUser {
    type Result = Result<Vec<User>, ServiceError>;
}

impl Message for PreRegister {
    type Result = Result<AuthRequest, ServiceError>;
}

impl Message for Login {
    type Result = Result<AuthResponse, ServiceError>;
}

impl Handler<GetUsers> for DatabaseService {
    type Result = ResponseFuture<Vec<User>, ServiceError>;

    fn handle(&mut self, msg: GetUsers, _: &mut Self::Context) -> Self::Result {
        Box::new(get_users(
            self.db.as_mut().unwrap(),
            self.users_by_id.as_ref().unwrap(),
            msg.0,
        ))
    }
}

impl Handler<PreRegister> for DatabaseService {
    type Result = ResponseFuture<AuthRequest, ServiceError>;

    fn handle(&mut self, msg: PreRegister, _: &mut Self::Context) -> Self::Result {
        let req = msg.0;
        let query = format!(
            "SELECT username, email FROM users
             WHERE username='{}' OR email='{}'", req.username, req.email.as_ref().unwrap());

        Box::new(simple_query(self.db.as_mut().unwrap(), &query)
            .and_then(|msg| unique_username_email_check(&msg, req)))
    }
}

impl Handler<Register> for DatabaseService {
    type Result = ResponseFuture<Vec<User>, ServiceError>;

    fn handle(&mut self, msg: Register, _: &mut Self::Context) -> Self::Result {
        let req = msg.0;

        let hash = match hash::hash_password(&req.password) {
            Ok(hash) => hash,
            Err(e) => return Box::new(future::err(e))
        };
        let id = match msg.1.lock() {
            Ok(mut var) => var.next_uid(),
            Err(_) => return Box::new(future::err(ServiceError::InternalServerError))
        };
        let u = match req.make_user(&id, &hash) {
            Ok(u) => u,
            Err(e) => return Box::new(future::err(e))
        };
        let query = format!(
            "INSERT INTO users (id, username, email, hashed_password, avatar_url, signature)
             VALUES ('{}', '{}', '{}', '{}', '{}', '{}')
             RETURNING *", u.id, u.username, u.email, u.hashed_password, u.avatar_url, u.signature);

        Box::new(simple_query(self.db.as_mut().unwrap(), &query)
            .and_then(move |msg| user_from_msg(&msg).map(|u| vec![u])))
    }
}

impl Handler<Login> for DatabaseService {
    type Result = ResponseFuture<AuthResponse, ServiceError>;

    fn handle(&mut self, msg: Login, _: &mut Self::Context) -> Self::Result {
        let req = msg.0;
        let query = format!("SELECT * FROM users WHERE username='{}'", &req.username);

        Box::new(simple_query(self.db.as_mut().unwrap(), &query)
            .and_then(move |msg| auth_response_from_msg(&msg, &req.password)))
    }
}

impl Handler<UpdateUser> for DatabaseService {
    type Result = ResponseFuture<Vec<User>, ServiceError>;

    fn handle(&mut self, msg: UpdateUser, _: &mut Self::Context) -> Self::Result {
        let u = msg.0;

        let mut query = String::new();
        query.push_str("UPDATE users SET");

        if let Some(s) = u.username.as_ref() {
            let _ = write!(&mut query, " username='{}',", s);
        }
        if let Some(s) = u.avatar_url.as_ref() {
            let _ = write!(&mut query, " avatar_url='{}',", s);
        }
        if let Some(s) = u.signature.as_ref() {
            let _ = write!(&mut query, " signature='{}',", s);
        }
        if let Some(s) = u.show_created_at.as_ref() {
            let _ = write!(&mut query, " show_created_at='{}',", s);
        }
        if let Some(s) = u.show_email.as_ref() {
            let _ = write!(&mut query, " show_email='{}',", s);
        }
        if let Some(s) = u.show_updated_at.as_ref() {
            let _ = write!(&mut query, " show_updated_at='{}',", s);
        }
        if let Some(s) = u.is_admin.as_ref() {
            let _ = write!(&mut query, " is_admin='{}',", s);
        }
        if let Some(s) = u.blocked.as_ref() {
            let _ = write!(&mut query, " blocked='{}',", s);
        }
        if query.ends_with(",") {
            let _ = write!(&mut query, " updated_at=DEFAULT WHERE id='{}' RETURNING *", u.id.unwrap());
        } else {
            return Box::new(future::err(ServiceError::BadRequest));
        }

        Box::new(simple_query(self.db.as_mut().unwrap(), &query)
            .and_then(|msg| user_from_msg(&msg).map(|u| vec![u])))
    }
}