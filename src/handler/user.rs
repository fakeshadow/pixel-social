use futures::Future;

use actix_web::{web, HttpResponse, Error};
use diesel::prelude::*;

use crate::model::{
    errors::ServiceError,
    user::{User, AuthRequest, AuthResponse, UserQuery, UserUpdateRequest, ToUserRef},
    common::{PoolConnectionPostgres, GlobalGuard, Response, QueryOption, Validator, GetUserId, get_unique_id},
};
use crate::handler::cache::UpdateCache;
use crate::schema::users;
use crate::util::{hash, jwt};

type QueryResult = Result<HttpResponse, ServiceError>;

impl<'a> UserQuery<'a> {
    pub fn handle_query(self, opt: &QueryOption) -> QueryResult {
        let conn = &opt.db_pool.unwrap().get().unwrap();
        // ToDo: Find a better way to handle auth check.
        match self {
            UserQuery::GetMe(id) => get_user(Some(&id), None, opt),
            UserQuery::GetUser(id) => get_user(None, Some(&id), opt),
            UserQuery::Login(req) => {
                &self.check_login()?;
                login_user(&req, opt)
            }
            UserQuery::UpdateUser(req) => {
                if let Some(_) = req.username { &self.check_username()?; }
                update_user(&req, opt)
            }
            UserQuery::Register(req) => {
                &self.check_register()?;
                register_user(&req, opt)
            }
        }
    }
}

fn get_user(self_id: Option<&u32>, other_id: Option<&u32>, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get()?;
    let id = self_id.unwrap_or_else(|| other_id.unwrap());
    let user = users::table.find(&other_id.unwrap()).first::<User>(conn)?;

    let res = match self_id {
        Some(_) => HttpResponse::Ok().json(&user),
        None => HttpResponse::Ok().json(&user.to_ref())
    };
    let _ignore = UpdateCache::TopicPostUser(None, None, Some(&vec![user])).handle_update(opt.cache_pool);
    Ok(res)
}

fn login_user(req: &AuthRequest, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get()?;
    let user = users::table.filter(users::username.eq(&req.username)).first::<User>(conn)?;
    if user.blocked { return Err(ServiceError::Unauthorized); }

    hash::verify_password(&req.password, &user.hashed_password)?;

    let token = jwt::JwtPayLoad::new(user.id, user.is_admin).sign()?;
    Ok(HttpResponse::Ok().json(&AuthResponse { token: &token, user_data: &user.to_ref() }))
}

fn update_user(req: &UserUpdateRequest, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get()?;

    let user: User = diesel::update(users::table.filter(users::id.eq(&req.id))).set(req).get_result(conn)?;
    let _ignore = UpdateCache::TopicPostUser(None, None, Some(&vec![user])).handle_update(opt.cache_pool);

    Ok(Response::ModifiedUser.to_res())
}

fn register_user(req: &AuthRequest, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get()?;

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
            let id: u32 = opt.global_var.unwrap().lock()
                // ToDo: In case mutex guard failed change back to increment global vars directly.
                .map(|mut guarded_global_var| guarded_global_var.next_uid())
                .map_err(|_| ServiceError::InternalServerError)?;

            let user: User = diesel::insert_into(users::table).values(&req.make_user(&id, &password_hash)?).get_result(conn)?;
            let _ignore = UpdateCache::TopicPostUser(None, None, Some(&vec![user])).handle_update(opt.cache_pool);

            Ok(Response::Registered.to_res())
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