use std::fmt::Write;
use futures::{Future, future, IntoFuture};

use actix::prelude::*;
use chrono::NaiveDateTime;
use tokio_postgres::{Statement, Client, SimpleQueryMessage, SimpleQueryRow};

use crate::model::{
    actors::PostgresConnection,
    common::GlobalGuard,
    errors::ServiceError,
    user::{AuthRequest, AuthResponse, User, UpdateRequest},
};
use crate::handler::db::simple_query;
use crate::util::{hash, jwt};


pub struct GetUsers(pub Vec<u32>);

pub struct PreRegister(pub AuthRequest);

pub struct Register(pub AuthRequest, pub GlobalGuard);

pub struct Login(pub AuthRequest);

pub struct UpdateUser(pub UpdateRequest);

impl Message for GetUsers {
    type Result = Result<Vec<User>, ServiceError>;
}

impl Message for PreRegister {
    type Result = Result<AuthRequest, ServiceError>;
}

impl Message for Register {
    type Result = Result<Vec<User>, ServiceError>;
}

impl Message for Login {
    type Result = Result<AuthResponse, ServiceError>;
}

impl Message for UpdateUser {
    type Result = Result<Vec<User>, ServiceError>;
}

impl Handler<GetUsers> for PostgresConnection {
    type Result = ResponseFuture<Vec<User>, ServiceError>;

    fn handle(&mut self, msg: GetUsers, _: &mut Self::Context) -> Self::Result {
        Box::new(get_users(
            self.db.as_mut().unwrap(),
            self.users_by_id.as_ref().unwrap(),
            msg.0,
        ))
    }
}

impl Handler<PreRegister> for PostgresConnection {
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

impl Handler<Register> for PostgresConnection {
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
            .and_then(move |msg| user_from_msg(&msg)))
    }
}

impl Handler<Login> for PostgresConnection {
    type Result = ResponseFuture<AuthResponse, ServiceError>;

    fn handle(&mut self, msg: Login, _: &mut Self::Context) -> Self::Result {
        let req = msg.0;
        let query = format!("SELECT * FROM users WHERE username='{}'", &req.username);

        Box::new(simple_query(self.db.as_mut().unwrap(), &query)
            .and_then(move |msg| auth_response_from_msg(&msg, &req.password)))
    }
}

impl Handler<UpdateUser> for PostgresConnection {
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
            let i = query.len();
            query.remove(i - 1);
        } else {
            return Box::new(future::err(ServiceError::BadRequest));
        }
        let _ = write!(&mut query, " WHERE id='{}' RETURNING *", u.id.unwrap());

        Box::new(simple_query(self.db.as_mut().unwrap(), &query)
            .and_then(|msg| user_from_msg(&msg)))
    }
}

pub fn get_users(
    c: &mut Client,
    st: &Statement,
    ids: Vec<u32>,
) -> impl Future<Item=Vec<User>, Error=ServiceError> {
    let users = Vec::with_capacity(21);
    c.query(st, &[&ids])
        .from_err()
        .fold(users, move |mut users, row| {
            users.push(User {
                id: row.get(0),
                username: row.get(1),
                email: row.get(2),
                hashed_password: "1".to_owned(),
                avatar_url: row.get(4),
                signature: row.get(5),
                created_at: row.get(6),
                updated_at: row.get(7),
                is_admin: row.get(8),
                blocked: row.get(9),
                show_email: row.get(10),
                show_created_at: row.get(11),
                show_updated_at: row.get(12),
            });
            Ok::<_, ServiceError>(users)
        })
}

fn user_from_msg(
    opt: &Option<SimpleQueryMessage>
) -> Result<Vec<User>, ServiceError> {
    match opt {
        Some(msg) => match msg {
            SimpleQueryMessage::Row(row) => user_from_simple_row(row).map(|u| vec![u]),
            _ => Err(ServiceError::InternalServerError)
        }
        None => Err(ServiceError::InternalServerError)
    }
}

fn auth_response_from_msg(
    opt: &Option<SimpleQueryMessage>,
    pass: &str,
) -> Result<AuthResponse, ServiceError> {
    match opt {
        Some(msg) => match msg {
            SimpleQueryMessage::Row(row) => auth_response_from_simple_row(row, pass),
            _ => Err(ServiceError::InvalidUsername)
        }
        None => Err(ServiceError::InternalServerError)
    }
}

fn unique_username_email_check(
    opt: &Option<SimpleQueryMessage>,
    req: AuthRequest,
) -> Result<AuthRequest, ServiceError> {
    match opt {
        Some(msg) => match msg {
            SimpleQueryMessage::Row(row) => {
                let row = row.get(0).ok_or(ServiceError::InternalServerError)?;
                if row == &req.username {
                    Err(ServiceError::UsernameTaken)
                } else {
                    Err(ServiceError::EmailTaken)
                }
            }
            _ => Ok(req)
        }
        None => Err(ServiceError::BadRequest)
    }
}

fn auth_response_from_simple_row(
    row: &SimpleQueryRow,
    pass: &str,
) -> Result<AuthResponse, ServiceError> {
    let hash = row.get(3).ok_or(ServiceError::InternalServerError)?;
    let _ = hash::verify_password(pass, hash)?;

    let user = user_from_simple_row(row)?;
    let token = jwt::JwtPayLoad::new(user.id, user.is_admin).sign()?;

    Ok(AuthResponse { token, user })
}

fn user_from_simple_row(
    row: &SimpleQueryRow
) -> Result<User, ServiceError> {
    Ok(User {
        id: row.get(0).map(|s| s.parse::<u32>()).unwrap()?,
        username: row.get(1).ok_or(ServiceError::InternalServerError)?.to_owned(),
        email: row.get(2).ok_or(ServiceError::InternalServerError)?.to_owned(),
        hashed_password: row.get(3).ok_or(ServiceError::InternalServerError)?.to_owned(),
        avatar_url: row.get(4).ok_or(ServiceError::InternalServerError)?.to_owned(),
        signature: row.get(5).ok_or(ServiceError::InternalServerError)?.to_owned(),
        created_at: row.get(6).map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")).unwrap()?,
        updated_at: row.get(7).map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")).unwrap()?,
        is_admin: row.get(8).map(|s| s.parse::<u32>()).unwrap()?,
        blocked: if row.get(9) == Some("f") { false } else { true },
        show_email: if row.get(10) == Some("f") { false } else { true },
        show_created_at: if row.get(11) == Some("f") { false } else { true },
        show_updated_at: if row.get(12) == Some("f") { false } else { true },
    })
}


// future remove these functions related to diesel
use actix_web::web::block;
use diesel::prelude::*;
use crate::schema::users;

use crate::model::{
    common::{get_unique_id, GetUserId, PoolConnectionPostgres, PostgresPool},
};

pub fn get_unique_users<T>(
    vec: &Vec<T>,
    opt: Option<u32>,
    pool: &PostgresPool,
) -> impl Future<Item=Vec<User>, Error=ServiceError>
    where T: GetUserId {
    let ids = get_unique_id(vec, opt);
    let pool = pool.clone();
    block(move || get_users_by_id(&ids, &pool.get()?)).from_err()
}

pub fn get_users_by_id(ids: &Vec<u32>, conn: &PoolConnectionPostgres) -> Result<Vec<User>, ServiceError> {
    Ok(users::table.filter(users::id.eq_any(ids)).load::<User>(conn)?)
}

pub fn load_all_users(conn: &PoolConnectionPostgres) -> Result<Vec<User>, ServiceError> {
    Ok(users::table.load::<User>(conn)?)
}