use actix_web::{dev, FromRequest, HttpRequest};
use futures::FutureExt;
use tokio_postgres::types::Type;

use crate::handler::{cache::MyRedisPool, db::MyPostgresPool};
use crate::model::{
    common::GlobalVars,
    errors::ResError,
    user::{AuthRequest, AuthResponse, User},
};
use crate::util::jwt::JwtPayLoad;

pub type UserJwt = JwtPayLoad;

const USER_BY_NAME_EMAIL: &str = "SELECT * FROM users WHERE username=$1 OR email=$2";
const USER_BY_NAME: &str = "SELECT * FROM users WHERE username=$1";
const INSERT_USER: &str =
    "INSERT INTO users (id, username, email, hashed_password, avatar_url, signature)
    VALUES ($1, $2, $3, $4, $5, $6)
    RETURNING *";

const INSERT_USER_TYPES: &[Type; 6] = &[
    Type::OID,
    Type::VARCHAR,
    Type::VARCHAR,
    Type::VARCHAR,
    Type::VARCHAR,
    Type::VARCHAR,
];

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

impl MyPostgresPool {
    pub(crate) async fn register(
        &self,
        req: AuthRequest,
        g: &GlobalVars,
    ) -> Result<User, ResError> {
        let email = req
            .email
            .as_ref()
            .map(String::as_str)
            .ok_or(ResError::BadRequest)?;
        let username = req.username.as_str();

        let mut pool_ref = self.get_pool().await?;
        let mut cli = pool_ref.get_client();

        let st = cli.prepare(USER_BY_NAME_EMAIL).await?;
        let users: Vec<User> = cli
            .query_multi(&st, &[&username, &email], Vec::with_capacity(2))
            .await?;
        drop(pool_ref);

        for u in users.iter() {
            if u.username.as_str() == username {
                return Err(ResError::UsernameTaken);
            }
            if u.email.as_str() == email {
                return Err(ResError::EmailTaken);
            }
        }
        let hash = crate::util::hash::hash_password(req.password.as_str())?;

        let mut pool_ref = self.get_pool().await?;
        let mut cli = pool_ref.get_client();

        let st = cli.prepare_typed(INSERT_USER, INSERT_USER_TYPES).await?;

        let id = g.lock().map(|mut lock| lock.next_uid()).await;
        let u = req.make_user(id, hash.as_str())?;

        cli.query_one(
            &st,
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

    pub(crate) async fn login(&self, req: AuthRequest) -> Result<AuthResponse, ResError> {
        let mut pool_ref = self.get_pool().await?;
        let mut cli = pool_ref.get_client();

        let st = cli.prepare(USER_BY_NAME).await?;

        let user: User = cli.query_one(&st, &[&req.username]).await?;

        drop(pool_ref);

        crate::util::hash::verify_password(req.password.as_str(), user.hashed_password.as_str())?;

        let token = JwtPayLoad::new(user.id, user.privilege).sign()?;

        Ok(AuthResponse { token, user })
    }
}

impl MyRedisPool {
    pub async fn get_uid_from_uuid(&self, uuid: &str) -> Result<u32, ResError> {
        let hm = self.get_hash_map_brown(uuid).await?;
        Ok(hm
            .0
            .get("user_id")
            .ok_or(ResError::Unauthorized)?
            .parse::<u32>()?)
    }
}
