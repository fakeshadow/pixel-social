use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use actix_web::web;
use chrono::NaiveDateTime;
use diesel::{
    result::Error,
    prelude::*,
    pg::PgConnection,
    r2d2::{ConnectionManager, Pool as diesel_pool},
};
use r2d2_redis::{
    redis,
    r2d2 as redis_r2d2,
    RedisConnectionManager,
};

use crate::schema::{users, posts, topics};
use crate::util::validation as validate;


pub type PostgresPool = diesel_pool<ConnectionManager<PgConnection>>;
pub type RedisPool = redis_r2d2::Pool<RedisConnectionManager>;

// query option for handlers
pub struct QueryOption<'a> {
    pub db_pool: Option<&'a web::Data<PostgresPool>>,
    pub global_var: Option<&'a web::Data<GlobalGuard>>,
}


#[derive(Serialize)]
pub struct ResponseMessage<'a> {
    message: &'a str
}

impl<'a> ResponseMessage<'a> {
    pub fn new(msg: &'a str) -> Self {
        ResponseMessage {
            message: msg
        }
    }
}


pub trait GetSelfId {
    fn get_self_id(&self) -> &u32;
}

pub trait GetSelfCategory {
    fn get_self_category(&self) -> &u32;
}

pub trait GetSelfTimeStamp {
    fn get_last_reply_time(&self) -> &NaiveDateTime;
    fn get_last_reply_timestamp(&self) -> i64 {
        self.get_last_reply_time().timestamp()
    }
}

pub trait MatchUser {
    fn get_user_id(&self) -> &u32;

    // only add topic user_id when query for the first page of a topic. Other case just pass None in
    // capacity has to be changed along side with the limit constant in handlers.
    fn get_unique_id<'a, T>(items: &'a Vec<T>, topic_user_id: Option<&'a u32>) -> Vec<&'a u32>
        where T: MatchUser {
        let mut result: Vec<&u32> = Vec::with_capacity(21);
        let mut hash_set = HashSet::with_capacity(21);

        if let Some(user_id) = topic_user_id {
            result.push(user_id);
            hash_set.insert(user_id);
        }

        for item in items.iter() {
            if !hash_set.contains(item.get_user_id()) {
                result.push(item.get_user_id());
                hash_set.insert(item.get_user_id());
            }
        };
        result
    }

    fn match_user_index<T>(&self, users: &Vec<T>) -> Option<usize>
        where T: GetSelfId {
        let mut _index: Vec<usize> = Vec::with_capacity(1);
        for (index, user) in users.iter().enumerate() {
            if &self.get_user_id() == &user.get_self_id() {
                _index.push(index);
                break;
            }
        };
        if _index.len() == 0 { return None; }
        Some(_index[0])
    }

    // add user privacy filter here
    fn make_user_field<T>(&self, users: &Vec<T>) -> Option<T>
        where T: Clone + GetSelfId {
        match self.match_user_index(users) {
            Some(index) => Some(users[index].clone()),
            None => None
        }
    }
}

// need to improve validator with regex
pub trait Validator {
    fn get_username(&self) -> &str;
    fn get_password(&self) -> &str;
    fn get_email(&self) -> &str;

    fn check_username(&self) -> bool {
        let username = self.get_username();
        validate::validate_username(username)
    }

    fn check_password(&self) -> bool {
        let password = self.get_password();
        validate::validate_password(password)
    }

    fn check_email(&self) -> bool {
        let email = self.get_email();
        if !email.contains("@") { return false; }
        let email_str_vec: Vec<&str> = email.rsplitn(2, "@").collect();
        validate::validate_email(email_str_vec)
    }

    fn check_register(&self) -> bool {
        self.check_email() && self.check_password() && self.check_username()
    }

    fn check_login(&self) -> bool {
        self.check_password() && self.check_username()
    }
}


// type and struct for global vars
pub type GlobalGuard = Arc<Mutex<GlobalVar>>;

#[derive(Debug, Clone)]
pub struct GlobalVar {
    pub next_uid: u32,
    pub next_pid: u32,
    pub next_tid: u32,
}

impl GlobalVar {
    pub fn init(database_url: &str) -> Arc<Mutex<GlobalVar>> {
        let connection = PgConnection::establish(database_url).unwrap_or_else(|_| panic!("Failed to connect to database"));

        let last_uid = users::table.select(users::id).order(users::id.desc()).limit(1).load(&connection);
        let next_uid = match_id(last_uid);

        let last_pid = posts::table.select(posts::id).order(posts::id.desc()).limit(1).load(&connection);
        let next_pid = match_id(last_pid);

        let last_tid = topics::table.select(topics::id).order(topics::id.desc()).limit(1).load(&connection);
        let next_tid = match_id(last_tid);

        Arc::new(Mutex::new(GlobalVar {
            next_uid,
            next_pid,
            next_tid,
        }))
    }
}

fn match_id(last_id: Result<Vec<u32>, Error>) -> u32 {
    match last_id {
        Ok(id) => {
            if id.len() > 0 { id[0] + 1}
            else { 1 }
        }
        Err(_) => panic!("Database error.Failed to get ids")
    }
}
