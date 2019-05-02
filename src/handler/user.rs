use futures::Future;

use actix_web::{web, HttpResponse, Error};
use diesel::prelude::*;

use crate::model::{
    errors::ServiceError,
    user::{User, AuthRequest, AuthResponse, UserQuery, UserQueryResult, UserUpdateRequest, ToUserRef},
    common::{PoolConnectionPostgres, GlobalGuard, QueryOption, Validator, GetUserId, get_unique_id},
};
use crate::schema::users;
use crate::util::{hash, jwt};

type QueryResult = Result<HttpResponse, ServiceError>;

impl<'a> UserQuery<'a> {
    pub fn handle_query(self, opt: &QueryOption) -> QueryResult {
        let conn = &opt.db_pool.unwrap().get().unwrap();
        // ToDo: Find a better way to handle auth check.
        match self {
            UserQuery::GetMe(id) => get_me(&id, &conn),
            UserQuery::GetUser(name) => {
                &self.check_username()?;
                get_user(&name, &conn)
            }
            UserQuery::Login(req) => {
                &self.check_login()?;
                login_user(&req, &conn)
            }
            UserQuery::UpdateUser(req) => {
                if let Some(_) = req.username { &self.check_username()?; }
                update_user(&req, &conn)
            }
            UserQuery::Register(req) => {
                &self.check_register()?;
                register_user(&req, &opt.global_var, &conn)
            }
        }
    }
}

pub enum AsyncDb {
    GetMe(u32),
    GetUser(String),
}

pub fn async_query(query: AsyncDb, opt: &QueryOption) -> impl Future<Item=User, Error=Error> {
    let pool = opt.db_pool.unwrap().clone();
    web::block(move || {
        match query {
            AsyncDb::GetMe(id) => get_me_async(&id, pool.get()?),
            AsyncDb::GetUser(name) => get_user_async(&name, pool.get()?),
        }
    }).from_err()
}

fn get_me_async(id: &u32, conn: PoolConnectionPostgres) -> Result<User, ServiceError> {
    Ok(users::table.find(&id).first::<User>(&conn)?)
}

fn get_user_async(username: &str, conn: PoolConnectionPostgres) -> Result<User, ServiceError> {
    Ok(users::table.filter(users::username.eq(&username)).first::<User>(&conn)?)
}


fn get_me(id: &u32, conn: &PoolConnectionPostgres) -> QueryResult {
    let user = users::table.find(&id).first::<User>(conn)?;
    Ok(UserQueryResult::GotPublicUser(&user.to_ref()).to_response())
//    Ok(UserQueryResult::GotUser(&user).to_response())
}

fn get_user(username: &str, conn: &PgConnection) -> QueryResult {
    let user = users::table.filter(users::username.eq(&username)).first::<User>(conn)?;
    Ok(UserQueryResult::GotPublicUser(&user.to_ref()).to_response())
}

fn login_user(req: &AuthRequest, conn: &PgConnection) -> QueryResult {
    let user = users::table.filter(users::username.eq(&req.username)).first::<User>(conn)?;
    if user.blocked { return Err(ServiceError::Unauthorized); }

    hash::verify_password(&req.password, &user.hashed_password)?;

    let token = jwt::JwtPayLoad::new(user.id, user.is_admin).sign()?;
    Ok(UserQueryResult::LoggedIn(&AuthResponse { token: &token, user_data: &user.to_ref() }).to_response())
}

fn update_user(req: &UserUpdateRequest, conn: &PgConnection) -> QueryResult {
    let user = diesel::update(users::table.filter(users::id.eq(&req.id))).set(req).get_result(conn)?;
    Ok(UserQueryResult::GotUser(&user).to_response())
}

fn register_user(req: &AuthRequest, global_var: &Option<&GlobalGuard>, conn: &PgConnection) -> QueryResult {
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
            let id: u32 = global_var.unwrap().lock()
                // ToDo: In case mutex guard failed change back to increment global vars directly.
                .map(|mut guarded_global_var| guarded_global_var.next_uid())
                .map_err(|_| ServiceError::InternalServerError)?;

            diesel::insert_into(users::table).values(&req.make_user(&id, &password_hash)?).execute(conn)?;
            Ok(UserQueryResult::Registered.to_response())
        }
    }
}

/// helper query function
pub fn get_unique_users<T>(
    vec: &Vec<T>,
    opt: Option<u32>,
    conn: &PoolConnectionPostgres,
) -> Result<Vec<User>, ServiceError>
    where T: GetUserId {
    let user_ids = get_unique_id(&vec, opt);
    let users = users::table.filter(users::id.eq_any(&user_ids)).load::<User>(conn)?;
    Ok(users)
}

pub fn load_all_users(conn: &PoolConnectionPostgres) -> Result<Vec<User>, ServiceError> {
    Ok(users::table.load::<User>(conn)?)
}

pub fn get_last_uid(conn: &PoolConnectionPostgres) -> Result<Vec<u32>, ServiceError> {
    Ok(users::table.select(users::id).order(users::id.desc()).limit(1).load(conn)?)
}