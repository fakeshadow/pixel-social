use chrono::NaiveDateTime;
use diesel::{sql_query, prelude::*};

use crate::model::{
    errors::ServiceError,
    user::User,
    talk::{Talk, HistoryMessage, Create, Join},
    common::{PoolConnectionPostgres, match_id},
};

use crate::handler::user::get_users_by_id;
use crate::schema::talks;

pub fn get_history(
    table: &str,
    id: u32,
    time: &NaiveDateTime,
    conn: &PoolConnectionPostgres,
) -> Result<Vec<HistoryMessage>, ServiceError> {
    // ToDo: in case the query failed to get message history
    let query = format!("SELECT * FROM {}{} WHERE date <= {} ORDER BY date DESC LIMIT 40", table, id, time);
    Ok(sql_query(query).load(conn)?)
}

pub fn insert_message(
    table: &str,
    id: &u32,
    msg: &str,
    conn: &PoolConnectionPostgres,
) -> Result<(), ServiceError> {
    let query = format!("INSERT INTO {}{} (message) VALUES ({})", table, id, msg);
    let _ = sql_query(query).execute(conn)?;
    Ok(())
}

pub fn get_talk_members(id: u32, conn: &PoolConnectionPostgres) -> Result<Vec<User>, ServiceError> {
    let ids = talks::table.find(id).select(talks::users).first::<Vec<u32>>(conn)?;
    get_users_by_id(&ids, conn)
}

pub fn create_talk(
    msg: Create,
    conn: &PoolConnectionPostgres,
) -> Result<Talk, ServiceError> {
    let last_id = Ok(talks::table.select(talks::id).order(talks::id.desc()).limit(1).load(conn)?);

    let id = match_id(last_id);
    let talk = Talk::new(id, msg);

    let talk = diesel::insert_into(talks::table).values(&talk).get_result::<Talk>(conn)?;

    // ToDo: in case the query failed to generate new table
    let query = format!("CREATE TABLE talk{} (date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,message VARCHAR(512))", talk.id);

    let _ = sql_query(query).execute(conn)?;

    Ok(talk)
}

pub fn join_talk(
    msg: &Join,
    conn: &PoolConnectionPostgres,
) -> Result<(), ServiceError> {
    let mut users = talks::table.find(msg.talk_id).select(talks::users).first::<Vec<u32>>(conn)?;

    users.push(msg.talk_id);
    users.sort();

    let _ = diesel::update(talks::table.find(msg.talk_id))
        .set(talks::users.eq(users))
        .execute(conn)?;
    Ok(())
}

pub fn load_all_talks(
    conn: &PoolConnectionPostgres
) -> Result<Vec<Talk>, ServiceError> {
    Ok(talks::table.load::<Talk>(conn)?)
}