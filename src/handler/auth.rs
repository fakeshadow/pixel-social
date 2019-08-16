use actix_web::{dev, FromRequest, HttpRequest};

use futures::{
    future::{err as ft_err, Either},
    Future,
};

use crate::handler::{cache::CacheService, db::DatabaseService};
use crate::model::{
    common::GlobalVars,
    errors::ResError,
    user::{AuthRequest, AuthResponse, User},
};
use crate::util::jwt::JwtPayLoad;

pub type UserJwt = JwtPayLoad;

// jwt token extractor from request
impl FromRequest for JwtPayLoad {
    type Error = ResError;
    type Future = Result<UserJwt, ResError>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        match req.headers().get("Authorization") {
            Some(token) => {
                let vec: Vec<&str> = token
                    .to_str()
                    .unwrap_or("no token")
                    .rsplitn(2, ' ')
                    .collect();
                JwtPayLoad::from(vec[0])
            }
            None => Err(ResError::Unauthorized),
        }
    }
}

impl DatabaseService {
    pub fn check_register(
        &self,
        r: AuthRequest,
    ) -> impl Future<Item = AuthRequest, Error = ResError> {
        let query = format!(
            "SELECT username, email FROM users
             WHERE username='{}' OR email='{}'",
            r.username,
            r.email.as_ref().unwrap()
        );
        self.unique_username_email_check(query.as_str(), r)
    }

    pub fn register(
        &self,
        r: AuthRequest,
        g: &GlobalVars,
    ) -> impl Future<Item = User, Error = ResError> {
        let hash = match crate::util::hash::hash_password(&r.password) {
            Ok(hash) => hash,
            Err(e) => return Either::A(ft_err(e)),
        };
        let id = match g.lock() {
            Ok(mut var) => var.next_uid(),
            Err(_) => return Either::A(ft_err(ResError::InternalServerError)),
        };
        let u = match r.make_user(id, &hash) {
            Ok(u) => u,
            Err(e) => return Either::A(ft_err(e)),
        };

        use crate::handler::db::Query;
        Either::B(self.query_one_trait(
            &self.insert_user,
            &[
                &u.id,
                &u.username,
                &u.email,
                &u.hashed_password,
                &u.avatar_url,
                &u.signature,
            ],
        ))
    }

    pub fn login(&self, req: AuthRequest) -> impl Future<Item = AuthResponse, Error = ResError> {
        let query = format!("SELECT * FROM users WHERE username='{}'", &req.username);

        use crate::handler::db::SimpleQuery;
        self.simple_query_row_trait(query.as_str())
            .and_then(move |r| {
                let hash = r.get(3).ok_or(ResError::InternalServerError)?;
                let _ = crate::util::hash::verify_password(req.password.as_str(), hash)?;

                use std::convert::TryFrom;
                let user = User::try_from(r)?;
                let token = JwtPayLoad::new(user.id, user.privilege).sign()?;

                Ok(AuthResponse { token, user })
            })
    }
}

impl CacheService {
    pub fn get_uid_from_uuid(&self, uuid: &str) -> impl Future<Item = u32, Error = ResError> {
        self.get_hash_map(uuid).and_then(|hm| {
            Ok(hm
                .get("user_id")
                .ok_or(ResError::Unauthorized)?
                .parse::<u32>()?)
        })
    }
}
