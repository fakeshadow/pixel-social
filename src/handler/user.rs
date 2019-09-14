use std::future::Future;

use futures::compat::Future01CompatExt;
use futures01::Future as Future01;
use tokio_postgres::types::ToSql;

use crate::handler::{
    cache::{build_hmsets_01, CacheService, GetSharedConn, USER_U8},
    cache_update::CacheFailedMessage,
    db::{DatabaseService, GetDbClient, Query},
};
use crate::model::{
    errors::ResError,
    user::{UpdateRequest, User},
};

impl DatabaseService {
    pub async fn update_user(&self, u: UpdateRequest) -> Result<User, ResError> {
        let mut query = String::from("UPDATE users SET");
        let mut params = Vec::new();
        let mut index = 1u8;

        if let Some(s) = u.username.as_ref() {
            query.push_str(" username=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &dyn ToSql);
            index += 1;
        }

        if let Some(s) = u.avatar_url.as_ref() {
            query.push_str(" avatar_url=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &dyn ToSql);
            index += 1;
        }
        if let Some(s) = u.signature.as_ref() {
            query.push_str(" signature=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &dyn ToSql);
            index += 1;
        }
        if let Some(s) = u.show_email.as_ref() {
            query.push_str(" show_email=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &dyn ToSql);
            index += 1;
        }
        if let Some(s) = u.privilege.as_ref() {
            query.push_str(" privilege=$");
            query.push_str(index.to_string().as_str());
            params.push(s as &dyn ToSql);
            index += 1;
        }
        if query.ends_with(',') {
            query.pop();
            query.push_str(" WHERE id=$");
        } else {
            return Err(ResError::BadRequest);
        }

        query.push_str(index.to_string().as_str());
        params.push(u.id.as_ref().unwrap() as &dyn ToSql);

        query.push_str(" RETURNING *");

        let st = self.get_client().prepare(query.as_str()).await?;

        self.query_one(&st, &params).await
    }

    pub fn get_users_by_id(
        &self,
        ids: &[u32],
    ) -> impl Future<Output = Result<Vec<User>, ResError>> {
        self.query_multi(&self.users_by_id.borrow(), &[&ids], Vec::with_capacity(21))
    }
}

impl CacheService {
    pub fn get_users_from_ids(
        &self,
        mut ids: Vec<u32>,
    ) -> impl Future<Output = Result<Vec<User>, ResError>> {
        ids.sort();
        ids.dedup();
        use crate::handler::cache::UsersFromCache;
        self.users_from_cache_01(ids).compat()
    }

    pub fn update_users(&self, u: &[User]) {
        actix::spawn(build_hmsets_01(self.get_conn(), u, USER_U8, false).map_err(|_| ()));
    }

    pub fn update_user_return_fail01(
        &self,
        u: Vec<User>,
    ) -> impl Future01<Item = (), Error = Vec<User>> {
        build_hmsets_01(self.get_conn(), &u, USER_U8, true).map_err(|_| u)
    }

    pub fn send_failed_user(&self, u: Vec<User>) {
        let _ = self.recipient.do_send(CacheFailedMessage::FailedUser(u));
    }
}
