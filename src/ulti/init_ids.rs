use diesel::prelude::*;
use diesel::result::Error;

pub fn init(database_url: &str) -> Vec<u32> {
    use crate::schema::{
        users::{columns::uid, dsl::users},
        posts::{columns::pid, dsl::posts},
        topics::{columns::tid, dsl::topics},
    };

    let connection = PgConnection::establish(database_url).unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    let last_uid = users.select(uid).order(uid.desc()).limit(1).load(&connection);
    let next_uid = match_id(last_uid);

    let last_pid = posts.select(pid).order(pid.desc()).limit(1).load(&connection);
    let next_pid = match_id(last_pid);

    let last_tid = topics.select(tid).order(tid.desc()).limit(1).load(&connection);
    let next_tid = match_id(last_tid);

    vec![next_uid, next_pid, next_tid]
}

fn match_id(last_id: Result<Vec<u32>, Error>) -> u32 {
    match last_id {
        Ok(id) => {
            if id.len() > 0 {
                id[0] + 1
            } else {
                1
            }
        }
        Err(err) => panic!("Database error.Failed to get ids")
    }
}