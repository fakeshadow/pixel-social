use diesel::{
    pg::PgConnection,
    prelude::*,
};
use r2d2_redis::{redis};

use crate::model::common::{GlobalVar, GlobalGuard, match_id};
use crate::schema::{posts, topics, users};

pub fn clear_cache(redis_url: &str) -> Result<usize, ()>{
    let redis_client = redis::Client::open(redis_url).unwrap();
    let clear_cache = redis_client.get_connection().unwrap();
    redis::cmd("flushall").query(&clear_cache).map_err(|_|())
}

pub fn init_global_var(database_url: &str) -> GlobalGuard {
    let connection = PgConnection::establish(database_url)
        .unwrap_or_else(|_| panic!("Failed to connect to database"));

    let last_uid = users::table
        .select(users::id)
        .order(users::id.desc())
        .limit(1)
        .load(&connection);
    let next_uid = match_id(last_uid);

    let last_pid = posts::table
        .select(posts::id)
        .order(posts::id.desc())
        .limit(1)
        .load(&connection);
    let next_pid = match_id(last_pid);

    let last_tid = topics::table
        .select(topics::id)
        .order(topics::id.desc())
        .limit(1)
        .load(&connection);
    let next_tid = match_id(last_tid);

    GlobalVar::new(next_uid, next_pid, next_tid)
}