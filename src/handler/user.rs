use actix_web::web;
use diesel::prelude::*;

use crate::model::{
	common::{GlobalGuard, PostgresPool, QueryOption},
	errors::ServiceError,
	user::*,
};
use crate::schema::users;
use crate::util::{hash, jwt};

pub fn user_handler(
	user_query: UserQuery,
	opt: QueryOption,
) -> Result<UserQueryResult, ServiceError> {
	let db_pool = opt.db_pool.unwrap();
	let conn: &PgConnection = &db_pool.get().unwrap();

	let select_user_column = (
		users::id,
		users::username,
		users::email,
		users::avatar_url,
		users::signature,
		users::created_at,
		users::updated_at,
	);

	match user_query {
		UserQuery::GetMe(my_id) => {
			let user: User = users::table
				.find(&my_id)
				.first::<User>(conn)?;

			Ok(UserQueryResult::GotUser(user))
		}

		UserQuery::GetUser(other_username) => {
			let user = users::table
				.filter(users::username.eq(&other_username))
				.select(select_user_column)
				.first::<SlimUser>(conn)?;

			Ok(UserQueryResult::GotSlimUser(user))
		}

		UserQuery::Login(login_request) => {
			let _username = login_request.username;
			let _password = login_request.password;

			let exist_user = users::table
				.filter(users::username.eq(&_username))
				.first::<User>(conn)?;

			if exist_user.blocked { return Err(ServiceError::Unauthorized); }

			let _check_password = hash::verify_password(&_password, &exist_user.hashed_password)?;

			let token = jwt::JwtPayLoad::new(exist_user.id).sign()?;

			Ok(UserQueryResult::LoggedIn(AuthResponse {
				token,
				user_data: exist_user.slim(),
			}))
		}

		UserQuery::UpdateUser(update_request) => {
			let user_id = update_request.id;

			let mut user_old = users::table.find(&user_id).first::<User>(conn)?;
			let user_new = update_request.update_user_data(&mut user_old)?;

			let updated_user = diesel::update(users::table.filter(users::id.eq(&user_id)))
				.set((
					users::username.eq(&user_new.username),
					users::avatar_url.eq(&user_new.avatar_url),
					users::signature.eq(&user_new.signature),
					users::is_admin.eq(&user_new.is_admin),
					users::blocked.eq(&user_new.blocked)
				))
				.get_result(conn)?;
			Ok(UserQueryResult::GotUser(updated_user))
		}

		UserQuery::Register(register_request) => {
			let _username = register_request.username;
			let _password = register_request.password;
			let _email = register_request.email;

			let exist_user: Vec<(String, String)> = users::table
				.select((users::username, users::email))
				.filter(users::username.eq(&_username))
				.or_filter(users::email.eq(&_email))
				.load(conn)?;

			if exist_user.len() > 0 {
				let (exist_username, _) = &exist_user[0];
				if exist_username == &_username {
					Err(ServiceError::UsernameTaken)
				} else {
					Err(ServiceError::EmailTaken)
				}
			} else {
				let password_hash: String = hash::hash_password(_password)?;
				let global_var = opt.global_var.unwrap();

				let id: u32 = match global_var.lock() {
					Ok(mut guarded_global_var) => {
						let next_uid = guarded_global_var.next_uid;
						guarded_global_var.next_uid += 1;
						next_uid
					}
					Err(_) => {
						return Err(ServiceError::InternalServerError);
					}
				};

				let user = User::new(id, &_username, &_email, &password_hash);
				diesel::insert_into(users::table)
					.values(&user)
					.execute(conn)?;
				Ok(UserQueryResult::Registered)
			}
		}
	}
}
