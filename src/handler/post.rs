use actix::Handler;
use diesel::prelude::*;

use crate::model::errors::ServiceError;

use crate::model::{
    post::{Post, IncomingPost},
    db::DbExecutor,
};

use crate::schema::posts::dsl::*;

impl Handler<IncomingPost> for DbExecutor {
    type Result = Result<(), ServiceError>;

    fn handle(&mut self, msg: IncomingPost, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();
        diesel::insert_into(posts)
            .values(&msg)
            .execute(conn)?;
        Ok(())
    }
}
