use std::sync::{Arc, Mutex};

use actix_web::{web::Data};

use crate::model::{
    errors::ServiceError,
    user::{ToUserRef, UserRef}};
use crate::util::validation as validate;


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

//ToDo: need to improve validator with regex
pub trait Validator {
    fn get_username(&self) -> &str;
    fn get_password(&self) -> &str;
    fn get_email(&self) -> &str;

    fn check_self_id(&self) -> Result<(), ServiceError>;

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
        &self.check_self_id()?;
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

#[derive(Clone, Debug)]
pub struct GlobalVar {
    pub last_uid: u32,
    pub last_pid: u32,
    pub last_tid: u32,
}

impl GlobalVar {
    pub fn new(last_uid: u32, last_pid: u32, last_tid: u32) -> GlobalGuard {
        Arc::new(Mutex::new(GlobalVar {
            last_uid,
            last_pid,
            last_tid,
        }))
    }
    pub fn next_uid(&mut self) -> u32 {
        self.last_uid += 1;
        self.last_uid
    }
    pub fn next_pid(&mut self) -> u32 {
        self.last_pid += 1;
        self.last_pid
    }
    pub fn next_tid(&mut self) -> u32 {
        self.last_tid += 1;
        self.last_tid
    }
}