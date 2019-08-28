use actix_web::{dev, FromRequest, HttpRequest};
use futures::{compat::Future01CompatExt, FutureExt};

use crate::handler::{
    cache::CacheService,
    db::{DatabaseService, SimpleQuery},
};
use crate::model::{
    common::GlobalVars,
    errors::ResError,
    user::{AuthRequest, AuthResponse, User},
};
use crate::util::jwt::JwtPayLoad;

pub type UserJwt = JwtPayLoad;

// use for req handlers use by both registered and anon guests.
pub struct UserJwtOpt(pub Option<JwtPayLoad>);

// jwt token extractor from request
impl FromRequest for JwtPayLoad {
    type Error = ResError;
    type Future = Result<UserJwt, ResError>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        match req.headers().get("Authorization") {
            Some(h) => {
                let vec: Vec<&str> = h
                    .to_str()
                    .map_err(|_| ResError::ParseError)?
                    .rsplitn(2, ' ')
                    .collect();
                JwtPayLoad::from(vec.get(0).ok_or(ResError::Unauthorized)?)
            }
            None => Err(ResError::Unauthorized),
        }
    }
}

impl FromRequest for UserJwtOpt {
    type Error = ();
    type Future = Result<UserJwtOpt, ()>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        if let Some(h) = req.headers().get("Authorization") {
            if let Ok(h) = h.to_str() {
                let h: Vec<&str> = h.rsplitn(2, ' ').collect();
                if let Some(h) = h.get(0) {
                    return Ok(UserJwtOpt(JwtPayLoad::from(h).ok()));
                }
            }
        }
        Ok(UserJwtOpt(None))
    }
}

impl DatabaseService {
    pub async fn check_register(&self, req: &AuthRequest) -> Result<(), ResError> {
        let username = req.username.as_str();
        let query = format!(
            "SELECT username, email FROM users
             WHERE username='{}' OR email='{}'",
            username,
            req.email.as_ref().unwrap()
        );

        match self.simple_query_row_trait(query.as_str()).await {
            Ok(row) => {
                if let Some(name) = row.get(0) {
                    if name == username {
                        return Err(ResError::UsernameTaken);
                    } else {
                        return Err(ResError::EmailTaken);
                    }
                }
                Ok(())
            }
            Err(e) => {
                if let ResError::NoContent = e {
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }

    pub async fn register(&self, r: AuthRequest, g: &GlobalVars) -> Result<User, ResError> {
        let hash = crate::util::hash::hash_password(&r.password)?;

        let id = g.lock().map(|mut lock| lock.next_uid()).await;

        let u = r.make_user(id, &hash)?;

        use crate::handler::db::Query;
        self.query_one_trait(
            &self.insert_user.borrow(),
            &[
                &u.id,
                &u.username,
                &u.email,
                &u.hashed_password,
                &u.avatar_url,
                &u.signature,
            ],
        )
        .await
    }

    pub async fn login(&self, req: AuthRequest) -> Result<AuthResponse, ResError> {
        use std::convert::TryFrom;

        let query = format!("SELECT * FROM users WHERE username='{}'", &req.username);

        let row = self.simple_query_row_trait(query.as_str()).await?;
        let hash = row.get(3).ok_or(ResError::DataBaseReadError)?;

        crate::util::hash::verify_password(req.password.as_str(), hash)?;

        let user = User::try_from(row)?;
        let token = JwtPayLoad::new(user.id, user.privilege).sign()?;

        Ok(AuthResponse { token, user })
    }
}

impl CacheService {
    pub async fn get_uid_from_uuid(&self, uuid: &str) -> Result<u32, ResError> {
        use crate::handler::cache::HashMapBrownFromCache;
        let hm = self.hash_map_brown_from_cache_01(uuid).compat().await?;
        Ok(hm
            .0
            .get("user_id")
            .ok_or(ResError::Unauthorized)?
            .parse::<u32>()?)
    }
}
