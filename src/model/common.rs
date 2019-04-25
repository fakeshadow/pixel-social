use std::sync::{Arc, Mutex};

use actix_web::web;
use chrono::NaiveDateTime;
use diesel::{
    pg::PgConnection,
    r2d2::{ConnectionManager, Pool as diesel_pool},
    result::Error,
};
use r2d2_redis::{
    RedisConnectionManager,
    r2d2::Pool as redis_pool,
};

use crate::model::{
    errors::ServiceError,
    user::PublicUser,
};
use crate::util::validation as validate;
use crate::model::user::User;
use std::iter::FromIterator;

pub type PostgresPool = diesel_pool<ConnectionManager<PgConnection>>;
pub type RedisPool = redis_pool<RedisConnectionManager>;

pub struct QueryOption<'a> {
    pub db_pool: Option<&'a web::Data<PostgresPool>>,
    pub cache_pool: Option<&'a web::Data<RedisPool>>,
    pub global_var: Option<&'a web::Data<GlobalGuard>>,
}

impl<'a> QueryOption<'a> {
    pub fn new(
        db_pool: Option<&'a web::Data<PostgresPool>>,
        cache_pool: Option<&'a web::Data<RedisPool>>,
        global_var: Option<&'a web::Data<GlobalGuard>>,
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

pub trait SelfHaveField {
    fn have_topic(&self) -> bool;
    fn have_post(&self) -> bool;
}

pub trait GetSelfId {
    fn get_self_id(&self) -> &u32;
    fn get_self_id_copy(&self) -> u32;
}

/// trait for extract self user , self topic/post and self user_id from struct(Mainly topic/post with user).
pub trait GetSelfField<T, R>
    where T: GetSelfId {
    fn get_self_user(&self) -> Option<&T>;
    fn get_self_post_topic(&self) -> &R;
    fn get_self_user_id(&self) -> Option<u32> {
        match self.get_self_user() {
            Some(user) => Some(user.get_self_id_copy()),
            None => None
        }
    }
}

pub trait GetSelfCategory {
    fn get_self_category(&self) -> &u32;
}

pub trait GetSelfTimeStamp {
    fn get_last_reply_time(&self) -> &NaiveDateTime;
//    fn get_timescore(&self) -> i64 {
//        self.get_last_reply_time().timestamp_nanos() / 1000
//    }
}

pub trait AttachUser {
    type Output;
    fn get_user_id(&self) -> &u32;
    fn attach_from_raw(self, users: &Vec<User>) -> Self::Output;
    fn attach_from_public(self, users: &Vec<PublicUser>) -> Self::Output;

    // ToDo: same user can have multiple posts in the same vec so the data can't be moved. Need to find a way not cloning the userdata.
    fn make_user_from_raw(&self, users: &Vec<User>) -> Option<PublicUser> {
        users.iter().enumerate()
            .filter(|(index, user)|
                self.get_user_id() == user.get_self_id())
            .map(|(_, user)| user.clone().into()).collect::<Vec<PublicUser>>().pop()
    }
    fn make_user_from_public(&self, users: &Vec<PublicUser>) -> Option<PublicUser> {
        users.iter().enumerate()
            .filter(|(index, user)|
                self.get_user_id() == user.get_self_id())
            .map(|(_, user)| user).cloned().collect::<Vec<PublicUser>>().pop()
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
pub fn match_id(last_id: Result<Vec<u32>, Error>) -> u32 {
    match last_id {
        Ok(id) => {
            if id.len() > 0 { id[0] + 1 } else { 1 }
        }
        Err(_) => panic!("Database error.Failed to get ids"),
    }
}

/// only add topic user_id when query for the first page of a topic. Other case just pass None in
/// capacity has to be changed along side with the limit constant in handlers.
pub fn get_unique_id<'a, T>(items: &'a Vec<T>, topic_user_id: Option<&'a u32>) -> Vec<&'a u32>
    where T: AttachUser {
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
