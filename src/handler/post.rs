use actix_web::web;
use chrono::Utc;
use diesel::prelude::*;

use crate::model::{
	common::{PostgresPool, QueryOption, RedisPool},
	errors::ServiceError,
	post::*,
};
use crate::schema::{posts, topics};

pub fn post_handler(
	post_query: PostQuery,
	opt: QueryOption,
) -> Result<PostQueryResult, ServiceError> {
	let db_pool = opt.db_pool.unwrap();
	let conn: &PgConnection = &db_pool.get().unwrap();

	match post_query {
		PostQuery::GetPost(pid) => {
			let post = posts::table.find(&pid).first::<Post>(conn)?;
			Ok(PostQueryResult::GotPost(post))
		}

		PostQuery::AddPost(mut post_request) => {
			let now = Utc::now().naive_local();

			let to_topic = topics::table.filter(topics::id.eq(&post_request.topic_id));
			let update_data = (
				topics::last_reply_time.eq(&now),
				topics::reply_count.eq(topics::reply_count + 1),
			);
			let to_topic_check = diesel::update(to_topic).set(update_data).execute(conn)?;
			if to_topic_check == 0 { return Err(ServiceError::NotFound); }

			if let Some(pid) = post_request.post_id {
				let to_post = posts::table.filter(
					posts::id
						.eq(&pid)
						.and(posts::topic_id.eq(&post_request.topic_id)),
				);
				let update_data = (
					posts::last_reply_time.eq(&now),
					posts::reply_count.eq(posts::reply_count + 1),
				);
				let to_post_check = diesel::update(to_post).set(update_data).execute(conn)?;
				if to_post_check == 0 { post_request.post_id = None }
			}

			let global_var = opt.global_var.unwrap();
			let id: u32 = match global_var.lock() {
				Ok(mut guarded_global_var) => {
					let next_pid = guarded_global_var.next_pid;
					guarded_global_var.next_pid += 1;
					next_pid
				}
				Err(_) => {
					return Err(ServiceError::InternalServerError);
				}
			};

			let new_post = Post::new(id, post_request);

			diesel::insert_into(posts::table)
				.values(&new_post)
				.execute(conn)?;
			Ok(PostQueryResult::AddedPost)
		}

		PostQuery::UpdatePost(post_request) => {


//			let old_post = posts::table.filter(
//				posts::id
//					.eq(&post_request.id)
//					.and(posts::user_id.eq(&post_request.user_id)),
//			);
//			let update_data = posts::post_content.eq(&post_request.post_content);
//
//			diesel::update(old_post).set(update_data).execute(conn)?;
			Ok(PostQueryResult::AddedPost)
		}
	}
}
