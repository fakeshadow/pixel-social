use chrono::NaiveDateTime;
use diesel::{sql_query, prelude::*};

use crate::model::{
    errors::ServiceError,
    user::User,
    talk::{Talk, HistoryMessage, Create, Join, Delete},
    common::{PoolConnectionPostgres, PostgresPool},
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
    pool: &PostgresPool,
) -> Result<(), ServiceError> {
    let conn = &pool.get()?;
    let query = format!("INSERT INTO {}{} (message) VALUES ({})", table, id, msg);
    let _ = sql_query(query).execute(conn)?;
    Ok(())
}

pub fn get_talk_members(
    id: u32,
    conn: &PoolConnectionPostgres,
) -> Result<Vec<User>, ServiceError> {
    let ids = talks::table.find(id).select(talks::users).first::<Vec<u32>>(conn)?;
    get_users_by_id(&ids, conn)
}

pub fn remove_talk_member(
    id: u32,
    talk_id: u32,
    pool: &PostgresPool,
) -> Result<(), ServiceError> {
    let conn = &pool.get()?;
    let mut ids: Vec<u32> = talks::table.find(talk_id).select(talks::users).first::<Vec<u32>>(conn)?;
    ids = remove_id(id, ids)?;

    let _ = diesel::update(talks::table.find(talk_id)).set(talks::users.eq(ids)).execute(conn)?;

    Ok(())
}

pub fn add_admin(
    id: u32,
    talk_id: u32,
    pool: &PostgresPool,
) -> Result<(), ServiceError> {
    let conn = &pool.get()?;
    let mut ids: Vec<u32> = talks::table.find(talk_id).select(talks::admin).first::<Vec<u32>>(conn)?;
    ids.push(id);
    ids.sort();
    let _ = diesel::update(talks::table.find(talk_id)).set(talks::admin.eq(ids)).execute(conn)?;
    Ok(())
}

pub fn remove_admin(
    id: u32,
    talk_id: u32,
    pool: &PostgresPool,
) -> Result<(), ServiceError> {
    let conn = &pool.get()?;
    let mut ids: Vec<u32> = talks::table.find(talk_id).select(talks::admin).first::<Vec<u32>>(conn)?;
    ids = remove_id(id, ids)?;

    let _ = diesel::update(talks::table.find(talk_id)).set(talks::admin.eq(ids)).execute(conn)?;

    Ok(())
}

pub fn create_talk(
    msg: &Create,
    conn: &PoolConnectionPostgres,
) -> Result<Talk, ServiceError> {
    let last_id:Vec<u32> = talks::table.select(talks::id).order(talks::id.desc()).limit(1).load(conn)?;

    let id = last_id.first().unwrap_or(&0) + 1;

    let array = vec![msg.owner];
    let query = format!(
        "INSERT INTO talks
        (id, name, description, owner, admin, users)
        VALUES ({}, '{}', '{}', {}, ARRAY {:?}, ARRAY {:?})",
        id,
        msg.name,
        msg.description,
        msg.owner,
        &array,
        &array);

    let _ = sql_query(query).execute(conn)?;

    let query = format!(
        "CREATE TABLE talk{}
        (date TIMESTAMP NOT NULL PRIMARY KEY DEFAULT CURRENT_TIMESTAMP,message VARCHAR(512))",
        id);

    let _ = sql_query(query).execute(conn)?;

    // ToDo: fix problem getting returned talk when inserting.
    Ok(talks::table.find(id).first::<Talk>(conn)?)
}

pub fn remove_talk(
    msg: &Delete,
    conn: &PoolConnectionPostgres,
) -> Result<(), ServiceError> {
    let query = format!("DROP TABLE talk{}", msg.talk_id);
    sql_query(query).execute(conn)?;
    Ok(())
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

pub fn remove_id(
    id: u32,
    mut ids: Vec<u32>,
) -> Result<Vec<u32>, ServiceError> {
    let (index, _) = ids
        .iter()
        .enumerate()
        .filter(|(i, uid)| *uid == &id)
        .next()
        .ok_or(ServiceError::InternalServerError)?;
    ids.remove(index);
    Ok(ids)
}