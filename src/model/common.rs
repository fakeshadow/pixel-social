use std::sync::{Arc, Mutex};

use actix_web::HttpResponse;
use diesel::{pg::PgConnection, r2d2::{ConnectionManager, Pool as diesel_pool, PooledConnection}};
use r2d2_redis::{RedisConnectionManager, r2d2::Pool as redis_pool};

use crate::model::{errors::ServiceError, user::{UserRef, ToUserRef}};
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

pub enum Response {
    Registered,
    ModifiedTopic,
    UpdatedCategory,
    AddedPost,
    ModifiedUser,
}

impl Response {
    pub fn to_res(&self) -> HttpResponse {
        match self {
            Response::Registered => HttpResponse::Ok().json(ResMsg::new("Register Success")),
            Response::ModifiedTopic => HttpResponse::Ok().json(ResMsg::new("Modify Topic Success")),
            Response::AddedPost => HttpResponse::Ok().json(ResMsg::new("Modify Post Success")),
            Response::UpdatedCategory => HttpResponse::Ok().json(ResMsg::new("Modify Category Success")),
            Response::ModifiedUser => HttpResponse::Ok().json(ResMsg::new("Modify User Success")),
        }
    }
}

#[derive(Serialize)]
struct ResMsg<'a> {
    message: &'a str,
}

impl<'a> ResMsg<'a> {
    pub fn new(msg: &'a str) -> Self {
        ResMsg { message: msg }
    }
}


pub trait GetSelfCategory {
    fn get_self_category(&self) -> &u32;
}

pub trait GetSelfId {
    fn get_self_id(&self) -> &u32;
}

pub trait AttachUser<'u, T>
    where T: GetSelfId + ToUserRef {
    type Output;
    fn self_user_id(&self) -> &u32;
    fn attach_user(&'u self, users: &'u Vec<T>) -> Self::Output;
    fn make_field(&self, users: &'u Vec<T>) -> Option<UserRef<'u>> {
        users.iter()
            .filter(|u| u.get_self_id() == self.self_user_id())
            .map(|u| u.to_ref())
            .next()
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
    fn get_user_id(&self) -> u32;
}

pub fn get_unique_id<T>(items: &Vec<T>, topic_user_id: Option<u32>) -> Vec<u32>
    where T: GetUserId {
    let mut result: Vec<u32> = Vec::with_capacity(21);

    if let Some(user_id) = topic_user_id { result.push(user_id); }

    for item in items.iter() {
        let item_id = item.get_user_id();
        if !result.contains(&item_id) {
            result.push(item_id);
        }
    }
    result
}
