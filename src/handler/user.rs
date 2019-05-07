use actix_web::{Error, HttpResponse, web};
use diesel::prelude::*;
use futures::Future;

use crate::handler::cache::{UpdateCache, MailCache};
use crate::model::{
    common::{get_unique_id, GetUserId, GlobalGuard, PoolConnectionPostgres, QueryOption, Response, Validator},
    errors::ServiceError,
    user::{AuthRequest, AuthResponse, ToUserRef, User, UserQuery, UserUpdateRequest},
};
use crate::schema::users;
use crate::util::{hash, jwt};


use crate::{model::mail::Mail, handler::email::send_mail};

type QueryResult = Result<HttpResponse, ServiceError>;

impl<'a> UserQuery<'a> {
    pub fn handle_query(&self, opt: &QueryOption) -> QueryResult {
        // ToDo: Find a better way to handle auth check.
        match self {
            UserQuery::GetMe(id) => get_user(Some(&id), None, opt),
            UserQuery::GetUser(id) => get_user(None, Some(&id), opt),
            UserQuery::Login(req) => {
                self.check_login()?;
                login_user(&req, opt)
            }
            UserQuery::UpdateUser(req) => {
                if let Some(_) = req.username { self.check_username()?; }
                update_user(&req, opt)
            }
            UserQuery::Register(req) => {
                self.check_register()?;
                register_user(&req, opt)
            }
        }
    }
}

fn get_user(self_id: Option<&u32>, other_id: Option<&u32>, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get()?;
    let id = self_id.unwrap_or_else(|| other_id.unwrap());
    let user = get_user_by_id(&id, conn)?.pop().ok_or(ServiceError::InternalServerError)?;

    let _ignore = UpdateCache::GotUser(&user).handle_update(&opt.cache_pool);

    Ok(match self_id {
        Some(_) => HttpResponse::Ok().json(&user),
        None => HttpResponse::Ok().json(&user.to_ref())
    })
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
    let update = req.make_update()?;
    let conn = &opt.db_pool.unwrap().get()?;

    let user: User = diesel::update(users::table.filter(users::id.eq(update.id)))
        .set(update).get_result(conn)?;
    let _ignore = UpdateCache::GotUser(&user).handle_update(&opt.cache_pool);

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
                .map(|mut guarded_global_var| guarded_global_var.next_uid())
                .map_err(|_| ServiceError::InternalServerError)?;

            let user = diesel::insert_into(users::table)
                .values(&req.make_user(&id, &password_hash)?)
                .get_result(conn)?;

            let _ignore = UpdateCache::GotUser(&user).handle_update(&opt.cache_pool);
            let _ignore = MailCache::AddActivation(user.to_mail()).modify(&opt.cache_pool);

            // ToDo: move sending mail to another thread.
            let mail_string = MailCache::GetActivation(Some(user.id)).get_mail_queue(&opt.cache_pool.unwrap())?;
            let mail = serde_json::from_str::<Mail>(&mail_string)?;
            let _ignore = send_mail(mail);

            Ok(Response::Registered.to_res())
        }
    }
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