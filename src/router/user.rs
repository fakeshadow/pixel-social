use actix_web::{web, HttpResponse};
use futures::IntoFuture;

use crate::handler::{auth::UserJwt, user::user_handler};
use crate::model::{
    common::{PostgresPool, QueryOption, RedisPool, ResponseMessage, Validator, GlobalGuard},
    errors::ServiceError,
    user::{UserQuery, AuthJson, AuthRequest, UserQueryResult, UserUpdateRequest},
};
use crate::util::validation::validate_username;
use crate::model::user::UserUpdateJson;

pub fn get_user(
    user_jwt: UserJwt,
    username: web::Path<String>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item = HttpResponse, Error = ServiceError> {
    if !validate_username(&username) {
        return Err(ServiceError::UsernameShort);
    }

    let name = username.to_string();
    let user_query = if &name == "me" {
        UserQuery::GetMe(&user_jwt.user_id)
    } else {
        UserQuery::GetUser(&name)
    };

    let opt = QueryOption {
        db_pool: Some(&db_pool),
        cache_pool: None,
        global_var: None,
    };

    match_query_result(user_handler(user_query, opt))
}

pub fn login_user(
    login_request: web::Json<AuthJson>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item = HttpResponse, Error = ServiceError> {
    if !login_request.check_login() {
        return Err(ServiceError::BadRequestGeneral);
    }
    let username = login_request.get_username();
    let password = login_request.get_password();

    let user_query = UserQuery::Login(AuthRequest {
        username,
        password,
        email: "",
    });

    let opt = QueryOption {
        db_pool: Some(&db_pool),
        cache_pool: None,
        global_var: None,
    };

    match_query_result(user_handler(user_query, opt))
}

pub fn update_user(
    user_jwt: UserJwt,
    update_request: web::Json<UserUpdateJson>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item = HttpResponse, Error = ServiceError> {
    if let Some(_) = update_request.username {
        if !update_request.check_username() {
            return Err(ServiceError::UsernameShort);
        }
    }

    let user_query = UserQuery::UpdateUser(UserUpdateRequest {
        id: Some(&user_jwt.user_id),
        username: update_request.username.as_ref(),
        avatar_url: update_request.avatar_url.as_ref(),
        signature: update_request.signature.as_ref(),
        is_admin: None,
        blocked: None,
    });

    let opt = QueryOption {
        db_pool: Some(&db_pool),
        cache_pool: None,
        global_var: None,
    };

    match_query_result(user_handler(user_query, opt))
}

//pub fn update_user_password((update_request, state): (Json<UserUpdateRequest>, State<AppState>))
//                            -> FutureResponse<HttpResponse> {
//    state.db
//        .send(UserQuery::UpdateUser(UserUpdateRequest {
//            id: None,
//            username: None,
//            password: None,
//            email: update_request.username.clone(),
//            avatar_url: None,
//            signature: None,
//            is_admin: None,
//            blocked: None,
//        }))
//        .from_err()
//        .and_then(|db_response| match db_response {
//            Ok(query_result) => Ok(Response::Modified(true).response()),
//            Err(service_error) => Ok(service_error.error_response()),
//        })
//        .responder()
//}

pub fn register_user(
    global_var: web::Data<GlobalGuard>,
    register_request: web::Json<AuthJson>,
    db_pool: web::Data<PostgresPool>,
) -> impl IntoFuture<Item = HttpResponse, Error = ServiceError> {
    if !register_request.check_register() {
        return Err(ServiceError::RegisterLimit);
    }
    let username = register_request.get_username();
    let email = register_request.get_email();
    let password = register_request.get_password();

    let user_query = UserQuery::Register(AuthRequest {
        username,
        email,
        password,
    });

    let opt = QueryOption {
        db_pool: Some(&db_pool),
        cache_pool: None,
        global_var: Some(&global_var),
    };

    match_query_result(user_handler(user_query, opt))
}

pub fn match_query_result(
    result: Result<UserQueryResult, ServiceError>,
) -> Result<HttpResponse, ServiceError> {
    match result {
        Ok(query_result) => match query_result {
            UserQueryResult::GotSlimUser(slim_user) => Ok(HttpResponse::Ok().json(slim_user)),
            UserQueryResult::GotUser(user) => Ok(HttpResponse::Ok().json(user)),
            UserQueryResult::LoggedIn(login_data) => Ok(HttpResponse::Ok().json(login_data)),
            UserQueryResult::Registered => {
                Ok(HttpResponse::Ok().json(ResponseMessage::new("Register Success")))
            }
        },
        Err(err) => Err(err),
    }
}
