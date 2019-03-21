use actix::Handler;
use diesel::prelude::*;

use crate::model::errors::ServiceError;
use crate::model::{post::*, db::DbExecutor};
use crate::schema::posts::dsl::*;

impl Handler<PostQuery> for DbExecutor {
    type Result = Result<PostQueryResult, ServiceError>;

    fn handle(&mut self, message: PostQuery, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();
        match message {
            PostQuery::GetPost(pid) => {
                match posts.find(&pid).first::<Post>(conn) {
                    Ok(post) => Ok(PostQueryResult::GotPost(post)),
                    Err(_) => {
                        Err(ServiceError::InternalServerError)
                    }
                }
            }
            PostQuery::AddPost(new_post) => {
                diesel::insert_into(posts)
                    .values(&new_post)
                    .execute(conn)?;
                Ok(PostQueryResult::AddedPost)
            }
        }
    }
}
