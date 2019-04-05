use actix_web::{web, HttpResponse};
use futures::IntoFuture;

use crate::model::{
	errors::ServiceError,
	admin::*,
	topic::{TopicQuery, TopicUpdateJson, TopicUpdateRequest},
	category::{CategoryQuery, CategoryUpdateRequest, CategoryUpdateJson},
	user::{UserQuery, UserQueryResult, UserUpdateJson, UserUpdateRequest},
	common::{ResponseMessage, PostgresPool, RedisPool, QueryOption},
};

use crate::router::{
	user::match_query_result as match_user_query_result,
	topic::match_query_result as match_topic_query_result,
	category::match_query_result as match_category_query_result,
};

use crate::handler::{
	cache::cache_handler,
	category::category_handler,
	admin::admin_handler,
	user::user_handler,
	post::post_handler,
	topic::topic_handler,
	auth::UserJwt,
};

pub fn admin_modify_category(
	user_jwt: UserJwt,
	update_request: web::Json<CategoryUpdateJson>,
	cache_pool: web::Data<RedisPool>,
	db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
	let opt = QueryOption {
		db_pool: Some(&db_pool),
		cache_pool: None,
		global_var: None,
	};

	let update_category_request = CategoryUpdateRequest {
		modify_type: &update_request.modify_type,
		category_id: update_request.category_id.as_ref(),
		category_name: update_request.category_name.as_ref(),
		category_theme: update_request.category_theme.as_ref(),
	};

// admin privilege check. need to improve for a complex level system.
	let admin_query = AdminQuery::UpdateCategoryCheck(&user_jwt.user_id, &update_category_request);
	let _checked = admin_handler(admin_query, &opt)?;

	let category_query = CategoryQuery::UpdateCategory(update_category_request);
	match_category_query_result(category_handler(category_query, opt), &cache_pool)
}


pub fn admin_update_user(
	user_jwt: UserJwt,
	update_request: web::Json<UserUpdateJson>,
	cache_pool: web::Data<RedisPool>,
	db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
	let opt = QueryOption {
		db_pool: Some(&db_pool),
		cache_pool: None,
		global_var: None,
	};

	let update_request = UserUpdateRequest {
		id: update_request.id.as_ref(),
		username: None,
		avatar_url: None,
		signature: None,
		is_admin: update_request.is_admin.as_ref(),
		blocked: update_request.blocked.as_ref(),
	};

// admin privilege check. need to improve for a complex level system.
	let admin_query = AdminQuery::UpdateUserCheck(&user_jwt.user_id, &update_request);
	let _checked = admin_handler(admin_query, &opt)?;

	let user_query = UserQuery::UpdateUser(update_request);

	match_user_query_result(user_handler(user_query, opt))
}

pub fn admin_update_topic(
	user_jwt: UserJwt,
	update_request: web::Json<TopicUpdateJson>,
	cache_pool: web::Data<RedisPool>,
	db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {

	let opt = QueryOption {
		db_pool: Some(&db_pool),
		cache_pool: None,
		global_var: None,
	};



	let update_request = TopicUpdateRequest {
		id: update_request.id.as_ref(),
		user_id: update_request.user_id.as_ref(),
		category_id: update_request.category_id.as_ref(),
		title: update_request.title.as_ref().map(String::as_str),
		body: update_request.body.as_ref().map(String::as_str),
		thumbnail: update_request.thumbnail.as_ref().map(String::as_str),
		is_locked: update_request.is_locked.as_ref(),
	};

	let admin_query = AdminQuery::UpdateTopicCheck(&user_jwt.user_id, &update_request);
	let _checked = admin_handler(admin_query, &opt)?;

	let topic_query = TopicQuery::UpdateTopic(update_request);

	match_topic_query_result(topic_handler(topic_query, opt), &cache_pool)

}
