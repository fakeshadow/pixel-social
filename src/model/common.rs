use std::sync::Arc;

use actix::prelude::Addr;
use async_std::sync::RwLock;
use hashbrown::HashMap;

use crate::model::{actors::WsChatSession, errors::ResError, talk::Talk};
use crate::util::validation as validate;

pub type BoxedFuture01Result<T> = Box<dyn futures01::Future<Item = T, Error = ResError>>;
pub type PinedBoxFutureResult<T> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, ResError>>>>;

pub trait GetSelfCategory {
    fn self_category(&self) -> u32;
}

pub trait SelfId {
    fn self_id(&self) -> u32;
}

pub trait SelfIdString {
    fn self_id_string(&self) -> String;
}

pub trait SelfUserId {
    fn get_user_id(&self) -> u32;
}

// ToDo: need to improve validator with regex
pub trait Validator
where
    Self: Sized,
{
    fn get_username(&self) -> &str;
    fn get_password(&self) -> &str;
    fn get_email(&self) -> &str;

    fn check_self_id(&self) -> Result<(), ResError>;

    fn check_username(&self) -> Result<(), ResError> {
        let username = self.get_username();
        if validate::validate_username(username) {
            Ok(())
        } else {
            Err(ResError::InvalidUsername)
        }
    }

    fn check_password(&self) -> Result<(), ResError> {
        let password = self.get_password();
        if validate::validate_password(password) {
            Ok(())
        } else {
            Err(ResError::InvalidPassword)
        }
    }

    fn check_email(&self) -> Result<(), ResError> {
        let email = self.get_email();
        if !email.contains('@') {
            return Err(ResError::InvalidEmail);
        }
        let email_str_vec: Vec<&str> = email.rsplitn(2, '@').collect();
        if validate::validate_email(email_str_vec) {
            Ok(())
        } else {
            Err(ResError::InvalidEmail)
        }
    }

    fn check_update(self) -> Result<Self, ResError> {
        self.check_self_id()?;
        self.check_username()?;
        Ok(self)
    }

    fn check_register(self) -> Result<Self, ResError> {
        self.check_email()?;
        self.check_password()?;
        self.check_username()?;
        Ok(self)
    }

    fn check_login(self) -> Result<Self, ResError> {
        self.check_password()?;
        self.check_username()?;
        Ok(self)
    }
}

// type and struct for global vars
// use future aware mutex for globals
pub type GlobalVars = futures::lock::Mutex<GlobalVar>;
pub type GlobalTalks = Arc<RwLock<HashMap<u32, Talk>>>;
pub type GlobalSessions = Arc<RwLock<HashMap<u32, Addr<WsChatSession>>>>;

pub fn new_global_talks_sessions(talks_vec: Vec<Talk>) -> (GlobalTalks, GlobalSessions) {
    let mut talks = HashMap::new();

    for t in talks_vec.into_iter() {
        talks.insert(t.id, t);
    }

    (
        Arc::new(RwLock::new(talks)),
        Arc::new(RwLock::new(HashMap::new())),
    )
}

#[derive(Debug)]
pub struct GlobalVar {
    pub last_uid: u32,
    pub last_pid: u32,
    pub last_tid: u32,
    pub last_cid: u32,
}

impl GlobalVar {
    pub fn new(last_uid: u32, last_pid: u32, last_tid: u32, last_cid: u32) -> GlobalVars {
        futures::lock::Mutex::new(GlobalVar {
            last_uid,
            last_pid,
            last_tid,
            last_cid,
        })
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
    pub fn next_cid(&mut self) -> u32 {
        self.last_cid += 1;
        self.last_cid
    }
}
