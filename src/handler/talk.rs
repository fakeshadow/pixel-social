use diesel::prelude::*;

use crate::model::{
    errors::ServiceError,
    user::User,
    talk::{Talk, Create},
    common::{PoolConnectionPostgres, match_id},
};

use crate::handler::user::get_users_by_id;
use crate::schema::talks;

pub fn get_room_members(id: u32, conn: &PoolConnectionPostgres) -> Result<Vec<User>, ServiceError> {
    let ids = talks::table.find(id).select(talks::users).first::<Vec<u32>>(conn)?;

    get_users_by_id(&ids, conn)
}

pub fn create_talk(msg: Create, conn: &PoolConnectionPostgres) -> Result<Talk, ServiceError> {
    let last_id = Ok(talks::table
        .select(talks::id).order(talks::id.desc()).limit(1).load(conn)?);
    let id = match_id(last_id);
    let talk = Talk::new(id, msg);
    Ok(diesel::insert_into(talks::table).values(&talk).get_result(conn)?)
}