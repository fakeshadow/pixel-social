use std::future::Future;

use tokio_postgres::types::ToSql;

use crate::handler::{
    cache::MyRedisPool,
    cache::USER_U8,
    cache_update::{CacheFailedMessage, SharedCacheUpdateAddr},
    db::MyPostgresPool,
};
use crate::model::{
    errors::ResError,
    user::{UpdateRequest, User},
};

impl MyPostgresPool {
    pub(crate) async fn update_user(&self, u: UpdateRequest) -> Result<User, ResError> {
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

        let mut pool = self.get_pool().await?;
        let mut cli = pool.get_client();

        let st = cli.prepare(query.as_str()).await?;
        cli.query_one(&st, params.as_slice()).await
    }

    pub(crate) async fn get_users(&self, ids: &[u32]) -> Result<Vec<User>, ResError> {
        let mut pool = self.get_pool().await?;
        let (mut cli, sts) = pool.get_client_statements();

        let st = sts.get_statement(2)?;
        cli.query_multi(st, &[&ids], Vec::with_capacity(21)).await
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

    pub(crate) async fn update_user_send_fail(
        &self,
        u: User,
        addr: SharedCacheUpdateAddr,
    ) -> Result<(), ()> {
        let id = u.id;
        let r = self.build_sets(&[u], USER_U8, true).await;
        if r.is_err() {
            addr.do_send(CacheFailedMessage::FailedUser(id)).await;
        };
        Ok(())
    }
}
