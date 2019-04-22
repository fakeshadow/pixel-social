//use actix_web::{web, Error, HttpResponse, ResponseError};
//use futures::{IntoFuture, Future};
//
//use crate::handler::{
//    auth::UserJwt,
//    category::category_handler_test,
//    topic::topic_handler,
//    user::user_handler};
//use crate::model::{
//    user::*,
//    cache::*,
//    category::*,
//    topic::*,
//    common::{GlobalGuard, PostgresPool, QueryOption, RedisPool, ResponseMessage},
//    errors::ServiceError,
//};
//
//pub fn test_global_var(
//    global_var: web::Data<GlobalGuard>,
//    db_pool: web::Data<PostgresPool>,
//) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
//
//    let user_id = &1;
//    let category_id = &1;
//    let thumbnail = "test thumbnail";
//    let title = "test title";
//    let body = "test body";
//
//    let topic_query = TopicQuery::AddTopic(NewTopicRequest {
//        user_id,
//        category_id,
//        thumbnail,
//        title,
//        body,
//    });
//
//    let opt = QueryOption {
//        db_pool: Some(&db_pool),
//        cache_pool: None,
//        global_var: Some(&global_var),
//    };
//
//    match_query_result(topic_handler(topic_query, opt))
//}
//
//// async test of db query. not good result for now
//pub fn get_category_async(
//    db_pool: web::Data<PostgresPool>,
//) -> impl Future<Item=HttpResponse, Error=ServiceError> {
//
//    let categories = vec![1];
//    let page = 1i64;
//    let category_request = CategoryRequestTest {
//        categories,
//        page,
//    };
//
//    let query = CategoryQueryTest::GetCategory(category_request);
//
//    category_handler_test(query, db_pool.clone())
//        .from_err()
//        .and_then(|result| match result {
//            CategoryQueryResult::GotCategories(categories) => {
//                Ok(HttpResponse::Ok().json(categories))
//            }
//            CategoryQueryResult::GotTopics(topics) => {
//                Ok(HttpResponse::Ok().json(topics))
//            }
//            CategoryQueryResult::UpdatedCategory => {
//                Ok(HttpResponse::Ok().json(ResponseMessage::new("Modify Success")))
//            }
//        })
//}
//
//pub fn generate_admin(
//    admin_user: web::Path<(String, String, String)>,
//    db_pool: web::Data<PostgresPool>,
//    global_var: web::Data<GlobalGuard>,
//) -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
//
//    let (username, password, email) = admin_user.as_ref();
//    let opt = QueryOption {
//        db_pool: Some(&db_pool),
//        cache_pool: None,
//        global_var: Some(&global_var),
//    };
//    let register_request = AuthRequest {
//        username: &username,
//        password: &password,
//        email: &email,
//    };
//    let user_query = UserQuery::Register(register_request);
//    let _register = user_handler(user_query, opt);
//    let opt = QueryOption {
//        db_pool: Some(&db_pool),
//        cache_pool: None,
//        global_var: None,
//    };
//    let user_query = UserQuery::GetUser(&username);
//    let user_id = match user_handler(user_query, opt) {
//        Ok(query_result) => match query_result {
//            UserQueryResult::GotSlimUser(user) => user.id,
//            _ => 0
//        },
//        Err(e) => return Err(e)
//    };
//    let update_request = UserUpdateRequest {
//        id: &user_id,
//        username: None,
//        avatar_url: None,
//        signature: None,
//        is_admin: Some(&9),
//        blocked: None,
//    };
//    let opt = QueryOption {
//        db_pool: Some(&db_pool),
//        cache_pool: None,
//        global_var: None,
//    };
//
//    let user_query = UserQuery::UpdateUser(update_request);
//    match user_handler(user_query, opt) {
//        Ok(result) => match result {
//            UserQueryResult::GotUser(user) => Ok(HttpResponse::Ok().json(user)),
//            _ => Ok(HttpResponse::Ok().finish())
//        },
//        Err(e) => Err(e)
//    }
//}
//
//fn match_query_result(
//    result: Result<TopicQueryResult, ServiceError>,
//) -> Result<HttpResponse, ServiceError> {
//    match result {
//        Ok(query_result) => match query_result {
//            TopicQueryResult::AddedTopic => {
//                Ok(HttpResponse::Ok().json(ResponseMessage::new("Add Topic Success")))
//            }
//            TopicQueryResult::GotTopicSlim(topic) => Ok(HttpResponse::Ok().json(topic)),
//        },
//        Err(e) => Err(e),
//    }
//}
