use futures::Future;

use actix_web::web::block;
use diesel::prelude::*;

use crate::model::{
    common::{get_unique_id, GetUserId, QueryOptAsync, Response, Validator, PoolConnectionPostgres},
    errors::ServiceError,
    user::{AuthRequest, UserUpdateRequest, AuthResponseAsync, User},
};
use crate::schema::users;
use crate::util::{hash, jwt};

pub enum UserQuery {
    GetUser(u32),
    Register(AuthRequest),
    Login(AuthRequest),
    UpdateUser(UserUpdateRequest),
    ValidationFailed(ServiceError),
}

type QueryResult = Result<User, ServiceError>;
type QueryResultMulti = Result<Vec<User>, ServiceError>;

impl UserQuery {
    pub fn into_user(self, opt: QueryOptAsync) -> impl Future<Item=User, Error=ServiceError> {
        block(move || match self {
            UserQuery::GetUser(id) => get_user(&id, opt),
            UserQuery::Register(req) => register_user(&req, opt),
            UserQuery::UpdateUser(req) => update_user(&req, opt),
            UserQuery::ValidationFailed(e) => Err(e),
            _ => panic!("only single object query can use into_user method")
        }).from_err()
    }
    pub fn into_login(self, opt: QueryOptAsync) -> impl Future<Item=AuthResponseAsync, Error=ServiceError> {
        block(move || match self {
            UserQuery::Login(req) => login_user(&req, opt),
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

pub fn get_users_async<T>(vec: &Vec<T>, opt: Option<u32>, conn: PoolConnectionPostgres)
                          -> impl Future<Item=Vec<User>, Error=ServiceError>
    where T: GetUserId {
    let ids = get_unique_id(vec, opt);
    block(move || Ok(users::table.filter(users::id.eq_any(&ids)).load::<User>(&conn)?)).from_err()
}

pub fn get_user_by_id_async(id: u32, conn: PoolConnectionPostgres) -> impl Future<Item=Vec<User>, Error=ServiceError> {
    block(move || Ok(users::table.find(&id).load::<User>(&conn)?)).from_err()
}


/// helper query function
pub fn get_unique_users<T>(vec: &Vec<T>, opt: Option<u32>, conn: &PoolConnectionPostgres)
                           -> Result<Vec<User>, ServiceError>
    where T: GetUserId {
    let user_ids = get_unique_id(&vec, opt);
    let users = users::table.filter(users::id.eq_any(&user_ids)).load::<User>(conn)?;
    Ok(users)
}

pub fn get_user_by_id(id: &u32, conn: &PoolConnectionPostgres) -> Result<Vec<User>, ServiceError> {
    Ok(users::table.find(&id).load::<User>(conn)?)
}

pub fn load_all_users(conn: &PoolConnectionPostgres) -> Result<Vec<User>, ServiceError> {
    Ok(users::table.load::<User>(conn)?)
}

pub fn get_last_uid(conn: &PoolConnectionPostgres) -> Result<Vec<u32>, ServiceError> {
    Ok(users::table.select(users::id).order(users::id.desc()).limit(1).load(conn)?)
}