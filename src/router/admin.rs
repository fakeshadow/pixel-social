use actix_web::{web, HttpResponse};
use futures::IntoFuture;

use crate::model::{
	errors::ServiceError,
	admin::*,
	post::{PostUpdateJson, PostUpdateRequest, PostQuery},
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
		category_id: update_request.category_id.as_ref(),
		category_name: update_request.category_name.as_ref().map(String::as_str),
		category_theme: update_request.category_theme.as_ref().map(String::as_str),
	};

	// admin privilege check. need to improve for a complex level system.
	let admin_query = AdminQuery::UpdateCategoryCheck(&user_jwt.is_admin, &update_category_request);
	admin_handler(admin_query, &opt)?;

	let category_query = match update_request.category_id {
		Some(_category_id) => CategoryQuery::UpdateCategory(update_category_request),
		None => CategoryQuery::AddCategory(update_category_request)
	};

	match_category_query_result(category_handler(category_query, opt), &cache_pool)
}

pub fn admin_remove_category(
	user_jwt: UserJwt,
	remove_request: web::Path<(u32)>,
	cache_pool: web::Data<RedisPool>,
	db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
	// need to add posts and topics migration along side the remove.
	let category_id = remove_request.into_inner();

	let opt = QueryOption {
		db_pool: Some(&db_pool),
		cache_pool: None,
		global_var: None,
	};

	let admin_query = AdminQuery::DeleteCategoryCheck(&user_jwt.is_admin, &category_id);
	admin_handler(admin_query, &opt)?;

	let category_query = CategoryQuery::DeleteCategory(&category_id);

	match_category_query_result(category_handler(category_query, opt), &cache_pool)
}

pub fn admin_update_user(
	user_jwt: UserJwt,
	update_request: web::Json<UserUpdateJson>,
//	cache_pool: web::Data<RedisPool>,
	db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
	let id  = match update_request.id {
		Some(id) => id,
		None => return Err(ServiceError::BadRequestGeneral)
	};

	let opt = QueryOption {
		db_pool: Some(&db_pool),
		cache_pool: None,
		global_var: None,
	};

	let update_request = UserUpdateRequest {
		id: &id,
		username: None,
		avatar_url: None,
		signature: None,
		is_admin: update_request.is_admin.as_ref(),
		blocked: update_request.blocked.as_ref(),
	};

// admin privilege check. need to improve for a complex level system.
	let admin_query = AdminQuery::UpdateUserCheck(&user_jwt.is_admin, &update_request);
	admin_handler(admin_query, &opt)?;

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

	let topic_request = TopicUpdateRequest {
		id: &update_request.id,
		user_id: None,
		category_id: update_request.category_id.as_ref(),
		title: update_request.title.as_ref().map(String::as_str),
		body: update_request.body.as_ref().map(String::as_str),
		thumbnail: update_request.thumbnail.as_ref().map(String::as_str),
		is_locked: update_request.is_locked.as_ref(),
	};

	let admin_query = AdminQuery::UpdateTopicCheck(&user_jwt.is_admin, &topic_request);
	admin_handler(admin_query, &opt)?;

	let topic_query = TopicQuery::UpdateTopic(topic_request);

	match_topic_query_result(topic_handler(topic_query, opt), &cache_pool)
}

pub fn admin_update_post(
	user_jwt: UserJwt,
	update_request: web::Json<PostUpdateJson>,
//	cache_pool: web::Data<RedisPool>,
	db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {

	let opt = QueryOption {
		db_pool: Some(&db_pool),
		cache_pool: None,
		global_var: None,
	};

	let post_request = PostUpdateRequest {
		id: &update_request.id,
		user_id: None,
		topic_id: update_request.topic_id.as_ref(),
		post_id: update_request.post_id.as_ref(),
		post_content: update_request.post_content.as_ref().map(String::as_str),
		is_locked: update_request.is_locked.as_ref(),
	};

	let admin_query = AdminQuery::UpdatePostCheck(&user_jwt.is_admin, &post_request);
	admin_handler(admin_query, &opt)?;

	Ok(HttpResponse::Ok().finish())
}
