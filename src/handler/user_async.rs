use futures::Future;

use actix_web::web::block;
use diesel::prelude::*;

use crate::handler::user::get_user_by_id;
use crate::model::{
    common::{get_unique_id, GetUserId, QueryOptAsync, Response, Validator},
    errors::ServiceError,
    user::{AuthRequest, UserUpdateRequest, AuthResponseAsync, User},
};
use crate::schema::users;
use crate::util::{hash, jwt};

pub enum UserQueryAsync {
    GetUser(u32),
    Register(AuthRequest),
    Login(AuthRequest),
    UpdateUser(UserUpdateRequest),
}

type QueryResult = Result<User, ServiceError>;
type QueryResultMulti = Result<Vec<User>, ServiceError>;

impl UserQueryAsync {
    pub fn into_user(self, opt: QueryOptAsync) -> impl Future<Item=User, Error=ServiceError> {
        block(move || match self {
            UserQueryAsync::GetUser(id) => get_user(&id, opt),
            UserQueryAsync::Register(req) => register_user(&req, opt),
            UserQueryAsync::UpdateUser(req) => update_user(&req, opt),
            _ => panic!("only single object query can use into_user method")
        }).from_err()
    }
    pub fn into_login(self, opt: QueryOptAsync) -> impl Future<Item=AuthResponseAsync, Error=ServiceError> {
        block(move || match self {
            UserQueryAsync::Login(req) => login_user(&req, opt),
            _ => panic!("only login query can use into_login method")
        }).from_err()
    }
}

fn get_user(id: &u32, opt: QueryOptAsync) -> QueryResult {
    let conn = &opt.db.unwrap().get()?;
    Ok(get_user_by_id(&id, &conn)?.pop().ok_or(ServiceError::InternalServerError)?)
}

fn update_user(req: &UserUpdateRequest, opt: QueryOptAsync) -> QueryResult {
    let update = req.make_update()?;
    let conn = &opt.db.unwrap().get()?;

    Ok(diesel::update(users::table.filter(users::id.eq(update.id)))
        .set(update).get_result(conn)?)
}

fn login_user(req: &AuthRequest, opt: QueryOptAsync) -> Result<AuthResponseAsync, ServiceError> {
    let conn = &opt.db.unwrap().get()?;
    let user = users::table.filter(users::username.eq(&req.username)).first::<User>(conn)?;
    if user.blocked { return Err(ServiceError::Unauthorized); }

    hash::verify_password(&req.password, &user.hashed_password)?;

    let token = jwt::JwtPayLoad::new(user.id, user.is_admin).sign()?;
    Ok(AuthResponseAsync { token, user })
}

fn register_user(req: &AuthRequest, opt: QueryOptAsync) -> QueryResult {
    let conn = &opt.db.unwrap().get()?;

    match users::table
        .select((users::username, users::email))
        .filter(users::username.eq(&req.username))
        .or_filter(users::email.eq(&req.extract_email()?))
        .load::<(String, String)>(conn)?.first() {
        Some((exist_username, _)) => if exist_username == &req.username {
            Err(ServiceError::UsernameTaken)
        } else {
            Err(ServiceError::EmailTaken)
        },
        None => {
            let password_hash: String = hash::hash_password(&req.password)?;
            let id: u32 = opt.global.unwrap().lock()
                .map(|mut guarded_global_var| guarded_global_var.next_uid())
                .map_err(|_| ServiceError::InternalServerError)?;

            Ok(diesel::insert_into(users::table)
                .values(&req.make_user(&id, &password_hash)?)
                .get_result(conn)?)
        }
    }
}