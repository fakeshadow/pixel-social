use actix_web::{web, Error, HttpResponse, ResponseError};
use futures::{IntoFuture, Future};

use crate::handler::auth::UserJwt;
use crate::model::{
    user::*,
    cache::*,
    category::*,
    topic::*,
    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool},
    errors::ServiceError,
};

pub fn test_global_var(
    global_var: web::Data<GlobalGuard>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let topic_query = TopicQuery::AddTopic(&NewTopicRequest {
        user_id: &1,
        category_id: &1,
        thumbnail: "test thumbnail",
        title: "test title",
        body: "test body",
    });
    let opt = QueryOption::new(Some(&db_pool), None, Some(&global_var));
    Ok(topic_query.handle_query(&opt)?.to_response())
}

pub fn generate_admin(
    admin_user: web::Path<(String, String, String)>,
    db_pool: web::Data<PostgresPool>,
    global_var: web::Data<GlobalGuard>,
) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    let (username, password, email) = admin_user.as_ref();

    let opt = QueryOption::new(Some(&db_pool), None, Some(&global_var));
    let register_request = AuthRequest {
        username,
        password,
        email: Some(email),
    };
    UserQuery::Register(&register_request).handle_query(&opt)?;
    let user_id = match UserQuery::GetUser(&username).handle_query(&opt) {
        Ok(query_result) => match query_result {
            UserQueryResult::GotSlimUser(user) => user.id,
            _ => 0
        },
        Err(e) => return Err(e)
    };
    let update_request = UserUpdateRequest {
        id: &user_id,
        username: None,
        avatar_url: None,
        signature: None,
        is_admin: Some(&9),
        blocked: None,
    };
    Ok(UserQuery::UpdateUser(&update_request).handle_query(&opt)?.to_response())
}
