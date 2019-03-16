use std::sync::{Arc, Mutex};

use diesel::prelude::*;

use crate::models::User;


pub fn init(database_url: &str) -> Vec<u32>{
    let mut vec = Vec::new();
    use crate::schema::users::{columns::uid as uid, dsl::users};
    let connection = PgConnection::establish(database_url).unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    let last_uid: Vec<u32> = users.select(uid).order(uid.desc()).limit(1).load(&connection).expect("Error Loading Users");
    println!("{:?}", last_uid[0]);

    vec.push(last_uid[0] + 1);
    vec
}