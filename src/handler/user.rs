use actix_web::{web, HttpResponse};
use diesel::prelude::*;

use crate::model::{
    errors::ServiceError,
    user::{User, AuthRequest, AuthResponse, UserQuery, UserQueryResult, UserUpdateRequest},
    common::{GlobalGuard, PostgresPool, QueryOption, Validator},
};
use crate::schema::users;
use crate::util::{hash, jwt};

type QueryResult = Result<HttpResponse, ServiceError>;

impl<'a> UserQuery<'a> {
    pub fn handle_query(self, opt: &QueryOption) -> QueryResult {
        let conn: &PgConnection = &opt.db_pool.unwrap().get().unwrap();
        // ToDo: Find a better way to handle auth check.
        match self {
            UserQuery::GetMe(id) => get_me(&id, &conn),
            UserQuery::GetUser(name) => {
                let _check = &self.check_username()?;
                get_user(&name, &conn)
            }
            UserQuery::Login(req) => {
                let _check = &self.check_login()?;
                login_user(&req, &conn)
            }
            UserQuery::UpdateUser(req) => {
                if let Some(_) = req.username { let _check = &self.check_username()?; }
                update_user(&req, &conn)
            }
            UserQuery::Register(req) => {
                let _check = &self.check_register()?;
                register_user(&req, &opt.global_var, &conn)
            }
        }
    }
}

fn get_me(id: &u32, conn: &PgConnection) -> QueryResult {
    let user: User = users::table.find(&id).first::<User>(conn)?;
    Ok(UserQueryResult::GotUser(user).to_response())
}

fn get_user(username: &str, conn: &PgConnection) -> QueryResult {
    let user = users::table
        .filter(users::username.eq(&username))
        .first::<User>(conn)?;
    Ok(UserQueryResult::GotPublicUser(user.into()).to_response())
}

fn login_user(req: &AuthRequest, conn: &PgConnection) -> QueryResult {
    let exist_user = users::table.filter(users::username.eq(&req.username)).first::<User>(conn)?;
    if exist_user.blocked { return Err(ServiceError::Unauthorized); }

    hash::verify_password(&req.password, &exist_user.hashed_password)?;

    let token = jwt::JwtPayLoad::new(exist_user.id, exist_user.is_admin).sign()?;
    Ok(UserQueryResult::LoggedIn(AuthResponse { token, user_data: exist_user.into() }).to_response())
}

fn update_user(req: &UserUpdateRequest, conn: &PgConnection) -> QueryResult {
    let updated_user = diesel::update(users::table
        .filter(users::id.eq(&req.id)))
        .set(req).get_result(conn)?;

    Ok(UserQueryResult::GotUser(updated_user).to_response())
}

fn register_user(req: &AuthRequest, global_var: &Option<&web::Data<GlobalGuard>>, conn: &PgConnection) -> QueryResult {
    match users::table
        .select((users::username, users::email))
        .filter(users::username.eq(&req.username))
        .or_filter(users::email.eq(&req.email.ok_or(ServiceError::BadRequestGeneral)?))
        .load::<(String, String)>(conn)?.pop() {
        Some((exist_username, _)) => if exist_username == req.username {
            Err(ServiceError::UsernameTaken)
        } else {
            Err(ServiceError::EmailTaken)
        },
        None => {
            let password_hash: String = hash::hash_password(req.password)?;
            let id: u32 = global_var.unwrap().lock()
                // ToDo: In case mutex guard failed change back to increment global vars directly.
                .map(|mut guarded_global_var| guarded_global_var.next_uid())
                .map_err(|_| ServiceError::InternalServerError)?;

            diesel::insert_into(users::table).values(&req.make_user(&id, &password_hash)).execute(conn)?;
            Ok(UserQueryResult::Registered.to_response())
        }
    }
}