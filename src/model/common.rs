use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use actix::prelude::Addr;
use hashbrown::HashMap;
use once_cell::sync::OnceCell;
use parking_lot::{Mutex, RwLock};

use crate::model::{actors::WsChatSession, errors::ResError, talk::Talk};
use crate::util::validation as validate;

pub const fn dur(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

pub const fn dur_as_sec(millis: u64) -> i64 {
    Duration::from_millis(millis).as_secs() as i64
}

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

// struct for global vars
#[derive(Clone, Default)]
pub struct GlobalTalks(pub Arc<RwLock<HashMap<u32, Talk>>>);

#[derive(Clone, Default)]
pub struct GlobalSessions(pub Arc<RwLock<HashMap<u32, Addr<WsChatSession>>>>);

pub fn global() -> &'static Mutex<GlobalVars> {
    static GLOBALS: OnceCell<Mutex<GlobalVars>> = OnceCell::new();
    GLOBALS.get_or_init(|| Mutex::new(Default::default()))
}

#[derive(Debug)]
pub struct GlobalVars {
    pub last_uid: u32,
    pub last_pid: u32,
    pub last_tid: u32,
    pub last_cid: u32,
}

impl Default for GlobalVars {
    fn default() -> Self {
        GlobalVars {
            last_uid: 0,
            last_pid: 0,
            last_tid: 0,
            last_cid: 0,
        }
    }
}

impl GlobalVars {
    pub fn update(&mut self, last_uid: u32, last_pid: u32, last_tid: u32, last_cid: u32) {
        self.last_uid = last_uid;
        self.last_pid = last_pid;
        self.last_tid = last_tid;
        self.last_cid = last_cid;
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

// could be unnecessary future.
pub struct OutOfOrder<'a, T>
where
    T: SelfId,
{
    ids: &'a [u32],
    vec: Vec<T>,
}

impl<'a, T: SelfId> OutOfOrder<'a, T> {
    pub(crate) fn sort(ids: &'a [u32], vec: Vec<T>) -> Self {
        OutOfOrder { ids, vec }
    }
}

impl<T> Future for OutOfOrder<'_, T>
where
    T: SelfId + Unpin,
{
    type Output = Vec<T>;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        let mut result = Vec::with_capacity(self.vec.len());
        let v = self.get_mut();

        for id in v.ids.iter() {
            for (i, idv) in v.vec.iter().enumerate() {
                if id == &idv.self_id() {
                    result.push(v.vec.swap_remove(i));
                    break;
                }
            }
        }
        Poll::Ready(result)
    }
}
