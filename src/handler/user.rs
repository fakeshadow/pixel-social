use std::future::Future;

use tokio_postgres::types::ToSql;

use crate::handler::{
    cache::MyRedisPool,
    cache::USER_U8,
    cache_update::{CacheFailedMessage, CacheServiceAddr},
    db::{GetStatement, MyPostgresPool, ParseRowStream},
};
use crate::model::{
    errors::ResError,
    user::{UpdateRequest, User},
};

impl MyPostgresPool {
    pub(crate) async fn update_user(&self, u: UpdateRequest) -> Result<Vec<User>, ResError> {
        let mut query = String::from("UPDATE users SET");
        let mut params = Vec::new();
        let mut index = 1u8;

        if let Some(s) = u.username.as_ref() {
            query.push_str(" username=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &(dyn ToSql + Sync));
            index += 1;
        }

        if let Some(s) = u.avatar_url.as_ref() {
            query.push_str(" avatar_url=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &(dyn ToSql + Sync));
            index += 1;
        }
        if let Some(s) = u.signature.as_ref() {
            query.push_str(" signature=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &(dyn ToSql + Sync));
            index += 1;
        }
        if let Some(s) = u.show_email.as_ref() {
            query.push_str(" show_email=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &(dyn ToSql + Sync));
            index += 1;
        }
        if let Some(s) = u.privilege.as_ref() {
            query.push_str(" privilege=$");
            query.push_str(index.to_string().as_str());
            params.push(s as &(dyn ToSql + Sync));
            index += 1;
        }
        if query.ends_with(',') {
            query.pop();
            query.push_str(" WHERE id=$");
        } else {
            return Err(ResError::BadRequest);
        }

        query.push_str(index.to_string().as_str());
        params.push(u.id.as_ref().unwrap() as &(dyn ToSql + Sync));

        query.push_str(" RETURNING *");

        let pool = self.get().await?;
        let (cli, _) = &*pool;

        let st = cli.prepare_typed(query.as_str(), &[]).await?;
        cli.query_raw(&st, params.iter().map(|s| *s as _))
            .await?
            .parse_row()
            .await
    }

    pub(crate) async fn get_users(&self, ids: &[u32]) -> Result<Vec<User>, ResError> {
        let pool = self.get().await?;
        let (cli, sts) = &*pool;

        let st = sts.get_statement("users_by_id")?;
        let params: [&(dyn ToSql + Sync); 1] = [&ids];

        cli.query_raw(st, params.iter().map(|s| *s as _))
            .await?
            .parse_row()
            .await
    }
}

impl MyRedisPool {
    pub(crate) fn get_users(
        &self,
        mut ids: Vec<u32>,
    ) -> impl Future<Output = Result<Vec<User>, ResError>> + '_ {
        ids.sort();
        ids.dedup();
        self.get_cache(ids, USER_U8, true)
    }

    pub(crate) async fn update_users(&self, u: &[User]) -> Result<(), ResError> {
        self.build_sets(u, USER_U8, false).await
    }

    pub(crate) async fn update_user_send_fail(&self, u: Vec<User>, addr: CacheServiceAddr) {
        let r = self.build_sets(&u, USER_U8, true).await;
        if r.is_err() {
            if let Some(id) = u.first().map(|u| u.id) {
                let _ = addr.send(CacheFailedMessage::FailedUser(id)).await;
            }
        };
    }
}
