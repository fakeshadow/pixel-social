use std::fmt::Write;

use futures::{
    future::{err as ft_err, Either},
    Future,
};

use crate::handler::{cache::CacheService, db::DatabaseService};
use crate::model::{
    errors::ResError,
    user::{UpdateRequest, User},
};

impl DatabaseService {
    pub fn update_user(&self, u: UpdateRequest) -> impl Future<Item=User, Error=ResError> {
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

        if query.ends_with(",") {
            let _ = write!(
                &mut query,
                " updated_at = DEFAULT WHERE id = {} RETURNING *",
                u.id.unwrap()
            );
        } else {
            return Either::A(ft_err(ResError::BadRequest));
        }

        use crate::handler::db::SimpleQuery;
        Either::B(self.simple_query_one_trait(query.as_str()))
    }
}

impl CacheService {
    pub fn get_users_from_ids(
        &self,
        mut ids: Vec<u32>,
    ) -> impl Future<Item=Vec<User>, Error=ResError> {
        ids.sort();
        ids.dedup();
        use crate::handler::cache::UsersFromCache;
        self.users_from_cache(ids)
    }
}
