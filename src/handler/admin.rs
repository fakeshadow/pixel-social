use actix_web::web;
use diesel::prelude::*;

use crate::model::{
	user::User,
	admin::AdminQuery,
	errors::ServiceError,
	common::{PostgresPool, RedisPool, QueryOption},
};
use crate::schema::users;

pub fn admin_handler(
	admin_query: AdminQuery,
	opt: &QueryOption,
) -> Result<(), ServiceError> {
	let db_pool = opt.db_pool.unwrap();
	let conn: &PgConnection = &db_pool.get().unwrap();

	match admin_query {
		AdminQuery::UpdateUserCheck(_self_user_id, _update_user_request) => {
			let admin_user: User = users::table.find(&_self_user_id).first::<User>(conn)?;
			let self_admin_level = &admin_user.is_admin;

			if !check_admin_level(_update_user_request.is_admin, &self_admin_level, 9) {
				return Err(ServiceError::Unauthorized);
			}

			let target_id = _update_user_request.id;
			let target_user: User = users::table.find(&target_id).first::<User>(conn)?;
			if self_admin_level <= &target_user.is_admin { return Err(ServiceError::Unauthorized); }

			Ok(())
		}
		AdminQuery::UpdateCategoryCheck(_self_user_id, _update_category_request) => {
			let admin_user: User = users::table.find(&_self_user_id).first::<User>(conn)?;
			let self_admin_level = &admin_user.is_admin;

			let modify_type = _update_category_request.modify_type;
			if self_admin_level < &2 { return Err(ServiceError::Unauthorized); }
			if self_admin_level < &9 && modify_type == &2 { return Err(ServiceError::Unauthorized); }

			Ok(())
		}
		AdminQuery::UpdateTopicCheck(_self_user_id, _update_topic_request) => {
			let admin_user: User = users::table.find(&_self_user_id).first::<User>(conn)?;
			let self_admin_level = &admin_user.is_admin;

			if !check_admin_level(_update_topic_request.title, &self_admin_level, 3) ||
				!check_admin_level(_update_topic_request.category_id, &self_admin_level, 3) ||
				!check_admin_level(_update_topic_request.body, &self_admin_level, 3) ||
				!check_admin_level(_update_topic_request.thumbnail, &self_admin_level, 3) ||
				!check_admin_level(_update_topic_request.is_locked, &self_admin_level, 2) {
				return Err(ServiceError::Unauthorized);
			}

			Ok(())
		}
	}
}

fn check_admin_level<T: ?Sized>(t: Option<&T>, self_level: &u32, target_level: u32) -> bool {
	if let Some(value) = t {
		if self_level < &target_level { return false; }
	}
	true
}
