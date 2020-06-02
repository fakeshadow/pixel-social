use actix_web::{dev, FromRequest, HttpRequest};
use futures::FutureExt;
use tokio_postgres::types::{ToSql, Type};

use crate::handler::{
    cache::MyRedisPool,
    db::{MyPostgresPool, ParseRowStream},
};
use crate::model::{
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
    type Future = futures::future::Ready<Result<UserJwt, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        futures::future::ready(extract_jwt(req))
    }
}

fn extract_jwt(req: &HttpRequest) -> Result<UserJwt, ResError> {
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

impl MyPostgresPool {
    pub(crate) async fn register(&self, req: AuthRequest) -> Result<Vec<User>, ResError> {
        let email = req.email.as_deref().ok_or(ResError::BadRequest)?;
        let username = req.username.as_str();

        let pool = self.get().await?;
        let (cli, _) = &*pool;

        let st = cli.prepare(USER_BY_NAME_EMAIL).await?;
        let params: [&(dyn ToSql + Sync); 2] = [&username, &email];
        let users = cli
            .query_raw(&st, params.iter().map(|s| *s as _))
            .await?
            .parse_row::<User>()
            .await?;

        drop(pool);

        for u in users.iter() {
            if u.username.as_str() == username {
                return Err(ResError::UsernameTaken);
            }
            if u.email.as_str() == email {
                return Err(ResError::EmailTaken);
            }
        }
        let hash = crate::util::hash::hash_password(req.password.as_str())?;

        let pool = self.get().await?;
        let (cli, _) = &*pool;

        let st = cli.prepare_typed(INSERT_USER, INSERT_USER_TYPES).await?;

        let id = crate::model::common::global()
            .lock()
            .map(|mut lock| lock.next_uid())
            .await;
        let u = req.make_user(id, hash.as_str())?;
        let params: [&(dyn ToSql + Sync); 6] = [
            &u.id,
            &u.username,
            &u.email,
            &u.hashed_password,
            &u.avatar_url,
            &u.signature,
        ];

        cli.query_raw(&st, params.iter().map(|s| *s as _))
            .await?
            .parse_row()
            .await
    }

    pub(crate) async fn login(&self, req: AuthRequest) -> Result<AuthResponse, ResError> {
        let pool = self.get().await?;
        let (cli, _) = &*pool;

        let st = cli.prepare_typed(USER_BY_NAME, &[]).await?;
        let params: [&(dyn ToSql + Sync); 1] = [&req.username];

        let user = cli
            .query_raw(&st, params.iter().map(|s| *s as _))
            .await?
            .parse_row::<User>()
            .await?
            .pop()
            .ok_or(ResError::PostgresError)?;

        drop(pool);

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
