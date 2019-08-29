use actix_web::{dev, FromRequest, HttpRequest};
use futures::{compat::Future01CompatExt, FutureExt};

use crate::handler::{
    cache::CacheService,
    db::{DatabaseService, GetDbClient, Query},
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
    pub async fn register(&self, req: AuthRequest, g: &GlobalVars) -> Result<User, ResError> {
        let st = self
            .get_client()
            .prepare("SELECT * FROM users WHERE username=$1 OR email=$2")
            .await?;

        let username = req.username.as_str();
        // unwrap() is safe as we checked the field in AuthRequest and make it's not none in router.
        let email = req.email.as_ref().map(String::as_str).unwrap();

        let users: Vec<User> = self
            .query_multi_trait(&st, &[&username, &email], Vec::with_capacity(2))
            .await?;

        for u in users.iter() {
            if u.username.as_str() == username {
                return Err(ResError::UsernameTaken);
            }
            if u.email.as_str() == email {
                return Err(ResError::EmailTaken);
            }
        }

        let hash = crate::util::hash::hash_password(req.password.as_str())?;

        let id = g.lock().map(|mut lock| lock.next_uid()).await;

        let u = req.make_user(id, hash.as_str())?;

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
        let st = self
            .get_client()
            .prepare("SELECT * FROM users WHERE username=$1")
            .await?;

        let user: User = self.query_one_trait(&st, &[&req.username]).await?;

        crate::util::hash::verify_password(req.password.as_str(), user.hashed_password.as_str())?;

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
