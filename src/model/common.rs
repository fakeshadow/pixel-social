use std::collections::HashSet;
use chrono::{Local, NaiveDateTime};

use crate::util::validation as validate;

pub trait GetSelfId {
    fn get_self_id(&self) -> &i32;
}

pub trait GetSelfCategory {
    fn get_self_category(&self) -> &i32;
}

pub trait GetSelfTimeStamp {
    fn get_last_reply_time(&self) -> &NaiveDateTime;
    fn get_last_reply_timestamp(&self) -> i64 {
        self.get_last_reply_time().timestamp()
    }
}



pub trait MatchUser {
    fn get_user_id(&self) -> &i32;

    // only add topic user_id when query for the first page of a topic. Other case just pass None in
    // capacity has to be changed along side with the limit constant in handlers.
    fn get_unique_id<'a, T>(items: &'a Vec<T>, topic_user_id: Option<&'a i32>) -> Vec<&'a i32>
        where T: MatchUser {
        let mut result: Vec<&i32> = Vec::with_capacity(21);
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