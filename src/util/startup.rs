use diesel::{
    pg::PgConnection,
    prelude::*,
};
use r2d2_redis::redis;

use crate::model::{
    category::Category,
    errors::ServiceError,
    common::{GlobalVar, GlobalGuard, match_id}
};
use crate::handler::{
    user::get_last_uid,
    post::get_last_pid,
    topic::get_last_tid
};
use crate::schema::categories;


pub fn clear_cache(redis_url: &str) -> Result<usize, ()> {
    let redis_client = redis::Client::open(redis_url).unwrap();
    let clear_cache = redis_client.get_connection().unwrap();
    redis::cmd("flushall").query(&clear_cache).map_err(|_| ())
}

// ToDo: Build category set, user set, topic rank at system start;

pub fn init_global_var(database_url: &str) -> GlobalGuard {
    let conn = PgConnection::establish(database_url)
        .unwrap_or_else(|_| panic!("Failed to connect to database"));

    let next_uid = match_id(get_last_uid(&conn));
    let next_pid = match_id(get_last_pid(&conn));
    let next_tid = match_id(get_last_tid(&conn));

    GlobalVar::new(next_uid, next_pid, next_tid)
}