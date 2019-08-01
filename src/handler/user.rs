use std::fmt::Write;

use actix::prelude::{
    Handler,
    Message,
    ResponseFuture,
};

use crate::{
    CacheService,
    DatabaseService
};
use crate::model::{
    errors::ResError,
    user::{User, UpdateRequest},
};

pub struct GetUsers(pub Vec<u32>);

impl Message for GetUsers {
    type Result = Result<Vec<User>, ResError>;
}

impl Handler<GetUsers> for DatabaseService {
    type Result = ResponseFuture<Vec<User>, ResError>;

    fn handle(&mut self, mut msg: GetUsers, _: &mut Self::Context) -> Self::Result {
        msg.0.sort();
        msg.0.dedup();

        Box::new(self.get_users_by_id(&msg.0))
    }
}


pub struct GetUsersCache(pub Vec<u32>);

impl Message for GetUsersCache {
    type Result = Result<Vec<User>, ResError>;
}

impl Handler<GetUsersCache> for CacheService {
    type Result = ResponseFuture<Vec<User>, ResError>;

    fn handle(&mut self, mut msg: GetUsersCache, _: &mut Self::Context) -> Self::Result {
        msg.0.sort();
        msg.0.dedup();
        Box::new(self.get_users_cache_from_ids(msg.0))
    }
}


pub struct UpdateUser(pub UpdateRequest);

impl Message for UpdateUser {
    type Result = Result<User, ResError>;
}

impl Handler<UpdateUser> for DatabaseService {
    type Result = ResponseFuture<User, ResError>;

    fn handle(&mut self, msg: UpdateUser, _: &mut Self::Context) -> Self::Result {
        let u = msg.0;

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
            let _ = write!(&mut query, " updated_at = DEFAULT WHERE id = {} RETURNING *", u.id.unwrap());
        } else {
            return Box::new(futures::future::err(ResError::BadRequest));
        }

        Box::new(self.simple_query_one(query.as_str()))
    }
}