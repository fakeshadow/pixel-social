use actix_web::web;
use diesel::prelude::*;

use crate::model::common::{PostgresPool, QueryOption};
use crate::model::errors::ServiceError;
use crate::model::{post::Post, topic::*, user::SlimUser};
use crate::schema::{categories, posts, topics, users};

const LIMIT: i64 = 20;

pub fn topic_handler(
	topic_query: TopicQuery,
	opt: QueryOption,
) -> Result<TopicQueryResult, ServiceError> {
	let db_pool = opt.db_pool.unwrap();
	let conn: &PgConnection = &db_pool.get().unwrap();

	match topic_query {
		TopicQuery::GetTopic(topic_id, page) => {
			let offset = (page - 1) * 20;

			let _topic: Topic = topics::table
				.filter(topics::id.eq(&topic_id))
				.first::<Topic>(conn)?;

			let _posts: Vec<Post> = posts::table
				.filter(posts::topic_id.eq(&_topic.id))
				.order(posts::id.asc())
				.limit(LIMIT)
				.offset(offset)
				.load::<Post>(conn)?;

			join_topics_users(_topic, _posts, conn, &page)
		}

		TopicQuery::AddTopic(new_topic_request) => {
			let cid = new_topic_request.category_id;

			let category_check: usize = categories::table.find(&cid).execute(conn)?;
			if category_check == 0 {
				return Err(ServiceError::NotFound);
			};

			let global_var = opt.global_var.unwrap();

			let id: u32 = match global_var.lock() {
				Ok(mut guarded_global_var) => {
					let next_tid = guarded_global_var.next_tid;
					guarded_global_var.next_tid += 1;
					next_tid
				}
				Err(_) => {
					return Err(ServiceError::InternalServerError);
				}
			};

			let new_topic = Topic::new(id, new_topic_request);

			diesel::insert_into(topics::table)
				.values(&new_topic)
				.execute(conn)?;
			Ok(TopicQueryResult::AddedTopic)
		}

		TopicQuery::UpdateTopic(topic_update_request) => {
			match topic_update_request.id {
				Some(tid) => {
					let mut topic_old = topics::table.find(&tid).first::<Topic>(conn)?;

					if let Some(user_id_check) = topic_update_request.user_id {
						if user_id_check != &topic_old.user_id {
							return Err(ServiceError::Unauthorized);
						}
					}

					let topic_new = topic_update_request.update_topic_data(&mut topic_old)?;

					diesel::update(topics::table.filter(topics::id.eq(&tid)))
						.set((
							topics::category_id.eq(&topic_new.category_id),
							topics::title.eq(&topic_new.title),
							topics::body.eq(&topic_new.body),
							topics::thumbnail.eq(&topic_new.thumbnail),
							topics::is_locked.eq(&topic_new.is_locked),
							topics::updated_at.eq(&diesel::dsl::now),
						))
						.execute(conn)?;
					Ok(TopicQueryResult::AddedTopic)
				}
				None => Err(ServiceError::BadRequestGeneral)
			}
		}
	}
}

fn join_topics_users(
	topic: Topic,
	posts: Vec<Post>,
	conn: &PgConnection,
	page: &i64,
) -> Result<TopicQueryResult, ServiceError> {
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
	let result = Post::get_unique_id(&posts, Some(topic.get_user_id()));

	let users: Vec<SlimUser> = users::table
		.filter(users::id.eq_any(&result))
		.select(&select_user_columns)
		.load::<SlimUser>(conn)?;

	let posts = posts
		.into_iter()
		.map(|post| post.attach_user(&users))
		.collect();
	let result = if page == &1 {
		TopicWithPost {
			topic: Some(topic.attach_user(&users)),
			posts: Some(posts),
		}
	} else {
		TopicWithPost {
			topic: None,
			posts: Some(posts),
		}
	};

	Ok(TopicQueryResult::GotTopicSlim(result))
}
