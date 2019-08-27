use std::fmt::Write;
use std::future::Future;

use futures::compat::Future01CompatExt;
use futures01::Future as Future01;

use crate::handler::{
    cache::{build_hmsets_01, CacheService, GetSharedConn, USER_U8},
    cache_update::CacheFailedMessage,
    db::DatabaseService,
};
use crate::model::{
    errors::ResError,
    user::{UpdateRequest, User},
};

impl DatabaseService {
    pub async fn update_user(&self, u: UpdateRequest) -> Result<User, ResError> {
        let mut query = String::new();
        query.push_str("UPDATE users SET");

        if let Some(s) = u.username.as_ref() {
            let _ = write!(&mut query, " username = '{}',", s);
        }
        if let Some(s) = u.avatar_url.as_ref() {
            let _ = write!(&mut query, " avatar_url = '{}',", s);
        }
        if let Some(s) = u.signature.as_ref() {
            let _ = write!(&mut query, " signature = '{}',", s);
        }
        if let Some(s) = u.show_email.as_ref() {
            let _ = write!(&mut query, " show_email = {},", s);
        }
        if let Some(s) = u.privilege.as_ref() {
            let _ = write!(&mut query, " privilege = {},", s);
        }

        if query.ends_with(',') {
            let _ = write!(
                &mut query,
                " updated_at = DEFAULT WHERE id = {} RETURNING *",
                u.id.unwrap()
            );
        } else {
            return Err(ResError::BadRequest);
        }

        use crate::handler::db::SimpleQuery;
        self.simple_query_one_trait(query.as_str()).await
    }

    pub fn get_users_by_id(&self, ids: &[u32]) -> impl Future<Output=Result<Vec<User>, ResError>> {
        use crate::handler::db::Query;
        self.query_multi_trait(&self.users_by_id.borrow(), &[&ids], Vec::with_capacity(21))
    }
}

impl CacheService {
    pub fn get_users_from_ids(
        &self,
        mut ids: Vec<u32>,
    ) -> impl Future<Output=Result<Vec<User>, ResError>> {
        ids.sort();
        ids.dedup();
        use crate::handler::cache::UsersFromCache;
        self.users_from_cache_01(ids).compat()
    }

    pub fn update_users(&self, u: &[User]) {
        actix::spawn(build_hmsets_01(self.get_conn(), u, USER_U8, false).map_err(|_| ()));
    }

    pub fn update_user_return_fail(&self, u: Vec<User>) -> impl Future01<Item=(), Error=Vec<User>> {
        build_hmsets_01(self.get_conn(), &u, USER_U8, true).map_err(|_| u)
    }

    pub fn send_failed_user(&self, u: Vec<User>) {
        let _ = self.recipient.do_send(CacheFailedMessage::FailedUser(u));
    }
}