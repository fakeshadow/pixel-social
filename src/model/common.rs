use std::sync::{Arc, Mutex};

use actix_web::web;
use chrono::NaiveDateTime;
use diesel::{
    pg::PgConnection,
    r2d2::{ConnectionManager, Pool as diesel_pool, PooledConnection},
};
use r2d2_redis::{
    RedisConnectionManager,
    r2d2::Pool as redis_pool,
};

use crate::model::{
    errors::ServiceError,
    user::{User, PublicUserRef, ToUserRef},
};
use crate::util::validation as validate;

pub type PostgresPool = diesel_pool<ConnectionManager<PgConnection>>;
pub type RedisPool = redis_pool<RedisConnectionManager>;
pub type PoolConnectionPostgres = PooledConnection<ConnectionManager<PgConnection>>;
pub type PoolConnectionRedis = PooledConnection<RedisConnectionManager>;

pub struct QueryOption<'a> {
    pub db_pool: Option<&'a PostgresPool>,
    pub cache_pool: Option<&'a RedisPool>,
    pub global_var: Option<&'a GlobalGuard>,
}

impl<'a> QueryOption<'a> {
    pub fn new(
        db_pool: Option<&'a PostgresPool>,
        cache_pool: Option<&'a RedisPool>,
        global_var: Option<&'a GlobalGuard>,
    ) -> QueryOption<'a> {
        QueryOption {
            db_pool,
            cache_pool,
            global_var,
        }
    }
}

#[derive(Serialize)]
pub struct ResponseMessage<'a> {
    message: &'a str,
}

impl<'a> ResponseMessage<'a> {
    pub fn new(msg: &'a str) -> Self {
        ResponseMessage { message: msg }
    }
}

pub trait GetSelfCategory {
    fn get_self_category(&self) -> &u32;
}

//pub trait GetSelfTimeStamp {
//    fn get_last_reply_time(&self) -> &NaiveDateTime;
//    fn get_timescore(&self) -> i64 {
//        self.get_last_reply_time().timestamp_nanos() / 1000
//    }
//}

pub trait GetSelfId {
    fn get_self_id(&self) -> &u32;
}

pub trait AttachUserRef<'u, T>
    where T: GetSelfId + ToUserRef {
    type Output;
    fn self_user_id(&self) -> &u32;
    fn attach_user(self, users: &'u Vec<T>) -> Self::Output;
    fn make_field(&self, users: &'u Vec<T>) -> Option<PublicUserRef<'u>> {
        let mut result: Vec<PublicUserRef> = Vec::with_capacity(1);
        for user in users.iter() {
            if self.self_user_id() == user.get_self_id() {
                result.push(user.to_ref());
                break ;
            }
        }
        result.pop()
    }
}

// need to improve validator with regex
pub trait Validator {
    fn get_username(&self) -> &str;
    fn get_password(&self) -> &str;
    fn get_email(&self) -> &str;

    fn check_username(&self) -> Result<(), ServiceError> {
        let username = self.get_username();
        if validate::validate_username(username) {
            Ok(())
        } else {
            Err(ServiceError::InvalidUsername)
        }
    }

    fn check_password(&self) -> Result<(), ServiceError> {
        let password = self.get_password();
        if validate::validate_password(password) {
            Ok(())
        } else {
            Err(ServiceError::InvalidPassword)
        }
    }

    fn check_email(&self) -> Result<(), ServiceError> {
        let email = self.get_email();
        if !email.contains("@") {
            return Err(ServiceError::InvalidEmail);
        }
        let email_str_vec: Vec<&str> = email.rsplitn(2, "@").collect();
        if validate::validate_email(email_str_vec) {
            Ok(())
        } else {
            Err(ServiceError::InvalidEmail)
        }
    }

    fn check_update(&self) -> Result<(), ServiceError> {
        &self.check_username()?;
        Ok(())
    }

    fn check_register(&self) -> Result<(), ServiceError> {
        &self.check_email()?;
        &self.check_password()?;
        &self.check_username()?;
        Ok(())
    }

    fn check_login(&self) -> Result<(), ServiceError> {
        &self.check_password()?;
        &self.check_username()?;
        Ok(())
    }
}

// type and struct for global vars
pub type GlobalGuard = Arc<Mutex<GlobalVar>>;

#[derive(Clone)]
pub struct GlobalVar {
    pub next_uid: u32,
    pub next_pid: u32,
    pub next_tid: u32,
}

impl GlobalVar {
    pub fn new(next_uid: u32, next_pid: u32, next_tid: u32) -> GlobalGuard {
        Arc::new(Mutex::new(GlobalVar {
            next_uid,
            next_pid,
            next_tid,
        }))
    }
    pub fn next_uid(&mut self) -> u32 {
        let id = self.next_uid;
        self.next_uid += 1;
        id
    }
    pub fn next_pid(&mut self) -> u32 {
        let id = self.next_pid;
        self.next_pid += 1;
        id
    }
    pub fn next_tid(&mut self) -> u32 {
        let id = self.next_tid;
        self.next_tid += 1;
        id
    }
}

// helper functions
pub fn match_id(last_id: Result<Vec<u32>, ServiceError>) -> u32 {
    match last_id {
        Ok(id) => {
            if id.len() > 0 { id[0] + 1 } else { 1 }
        }
        Err(_) => panic!("Database error.Failed to get ids"),
    }
}

/// only add topic user_id when query for the first page of a topic. Other case just pass None in
/// capacity has to be changed along side with the limit constant in handlers.
pub trait GetUserId {
    fn get_user_id(&self) -> &u32;
}

pub fn get_unique_id<'a, T>(items: &'a Vec<T>, topic_user_id: Option<&'a u32>) -> Vec<&'a u32>
    where T: GetUserId {
    let mut result: Vec<&u32> = Vec::with_capacity(21);

    if let Some(user_id) = topic_user_id { result.push(user_id); }

    for item in items.iter() {
        let item_id = item.get_user_id();
        if !result.contains(&item_id) {
            result.push(item_id);
        }
    }
    result
}
