use crate::model::{
    common::{PostgresPool, GlobalVar, GlobalGuard, match_id}
};
use crate::handler::{
    user::get_last_uid,
    post::get_last_pid,
    topic::get_last_tid,
};

// ToDo: Build category set, user set, topic rank at system start;

pub fn init_global_var(pool: &PostgresPool) -> GlobalGuard {
    let conn = pool.get().unwrap();
    let next_uid = match_id(get_last_uid(&conn));
    let next_pid = match_id(get_last_pid(&conn));
    let next_tid = match_id(get_last_tid(&conn));
    GlobalVar::new(next_uid, next_pid, next_tid)
}