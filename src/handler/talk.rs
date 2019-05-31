use diesel::prelude::*;

use crate::model::common::PoolConnectionPostgres;
use crate::model::errors::ServiceError;
use crate::model::user::User;

use crate::handler::user::get_users_by_id;

use crate::schema::talks;

pub fn get_room_members(id: u32, conn: &PoolConnectionPostgres) -> Result<Vec<User>, ServiceError> {
    let ids = talks::table.find(id).select(talks::users_id).first::<Vec<u32>>(conn)?;

    get_users_by_id(&ids, conn)
}