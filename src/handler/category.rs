use actix_web::web;
use diesel::prelude::*;

use crate::model::common::{PostgresPool, RedisPool, QueryOption, match_id};
use crate::model::errors::ServiceError;
use crate::model::{category::*, topic::*, user::SlimUser};
use crate::schema::{categories, topics, users};

const LIMIT: i64 = 20;


// async db test
use futures::future::{join_all, ok as fut_ok, Future};
use actix_http::error::DispatchError::Service;

pub fn category_handler_test(
	category_query: CategoryQueryTest,
	db_pool: web::Data<PostgresPool>,
) -> impl Future<Item=CategoryQueryResult, Error=ServiceError> {
	web::block(move || {
		let conn: &PgConnection = &db_pool.get().unwrap();
		match category_query {
			CategoryQueryTest::GetCategory(category_request) => {
				let page = category_request.page;
				let offset = (page - 1) * LIMIT;
				let categories_vec = category_request.categories;

				let _topics: Vec<Topic> = topics::table
					.filter(topics::category_id.eq_any(categories_vec))
					.order(topics::last_reply_time.desc())
					.limit(LIMIT)
					.offset(offset)
					.load::<Topic>(conn)?;

				join_topics_users(_topics, conn)
			}
		}
	})
		.from_err()
}

// sync db query
pub fn category_handler(
	category_query: CategoryQuery,
	opt: QueryOption,
) -> Result<CategoryQueryResult, ServiceError> {
	let db_pool = opt.db_pool.unwrap();
	let conn: &PgConnection = &db_pool.get().unwrap();

	match category_query {
		CategoryQuery::GetPopular(page) => {
			let offset = (page - 1) * LIMIT;

			let _topics: Vec<Topic> = topics::table
				.order(topics::last_reply_time.desc())
				.limit(LIMIT)
				.offset(offset)
				.load::<Topic>(conn)?;

			join_topics_users(_topics, conn)
		}

		CategoryQuery::GetCategory(category_request) => {
			let page = category_request.page;
			let offset = (page - 1) * LIMIT;
			let categories_vec = category_request.categories;

			let _topics: Vec<Topic> = topics::table
				.filter(topics::category_id.eq_any(categories_vec))
				.order(topics::last_reply_time.desc())
				.limit(LIMIT)
				.offset(offset)
				.load::<Topic>(conn)?;

			join_topics_users(_topics, conn)
		}

		CategoryQuery::GetAllCategories => {
			let categories_data = categories::table.load::<Category>(conn)?;
			Ok(CategoryQueryResult::GotCategories(categories_data))
		}

		CategoryQuery::AddCategory(category_request) => {
			let category_name = match category_request.category_name {
				Some(name) => name,
				None => return { Err(ServiceError::BadRequestGeneral) }
			};
			let category_theme = match category_request.category_theme {
				Some(theme) => theme,
				None => return { Err(ServiceError::BadRequestGeneral) }
			};
			let last_cid = categories::table.select(categories::id)
				.order(categories::id.desc())
				.limit(1)
				.load(conn);
			let next_cid = match_id(last_cid);

			let category_data = Category::new(next_cid, &category_name, &category_theme);

			diesel::insert_into(categories::table)
				.values(&category_data)
				.execute(conn)?;

			Ok(CategoryQueryResult::UpdatedCategory)
		}

		CategoryQuery::UpdateCategory(category_request) => {
			let target_category_id = match category_request.category_id {
				Some(id) => id,
				None => return Err(ServiceError::BadRequestGeneral)
			};

			let category_old_filter = categories::table
				.filter(categories::id.eq(&target_category_id));

			diesel::update(category_old_filter).set(&category_request.insert()).execute(conn)?;

			Ok(CategoryQueryResult::UpdatedCategory)
		}

		CategoryQuery::DeleteCategory(category_id) => {
			diesel::delete(categories::table.find(category_id))
				.execute(conn)?;

			Ok(CategoryQueryResult::UpdatedCategory)
		}
	}
}

fn join_topics_users(
	topics: Vec<Topic>,
	conn: &PgConnection,
) -> Result<CategoryQueryResult, ServiceError> {
	if topics.len() == 0 {
		return Ok(CategoryQueryResult::GotTopics(vec![]));
	};

	let select_user_columns = (
		users::id,
		users::username,
		users::email,
		users::avatar_url,
		users::signature,
		users::created_at,
		users::updated_at,
	);

	// use to bring the trait to scope
	use crate::model::common::MatchUser;
	let result = Topic::get_unique_id(&topics, None);

	let users: Vec<SlimUser> = users::table
		.filter(users::id.eq_any(&result))
		.select(&select_user_columns)
		.load::<SlimUser>(conn)?;

	Ok(CategoryQueryResult::GotTopics(
		topics
			.into_iter()
			.map(|topic| topic.attach_user(&users))
			.collect(),
	))
}
