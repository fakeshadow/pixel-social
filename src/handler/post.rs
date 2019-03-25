use actix::Handler;
use diesel::prelude::*;
use chrono::Utc;

use crate::model::errors::ServiceError;
use crate::model::{post::*, db::DbExecutor};
use crate::schema::posts;
use crate::schema::topics;

impl Handler<PostQuery> for DbExecutor {
    type Result = Result<PostQueryResult, ServiceError>;

    fn handle(&mut self, message: PostQuery, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();
        match message {
            PostQuery::GetPost(pid) => {
                match posts::table.find(&pid).get_result::<Post>(conn) {
                    Ok(post) => Ok(PostQueryResult::GotPost(post)),
                    Err(_) => {
                        Err(ServiceError::InternalServerError)
                    }
                }
            }

            PostQuery::AddPost(mut new_post) => {
                let now = Utc::now().naive_local();

                let to_topic = topics::table.filter(topics::id.eq(&new_post.topic_id));
                let update_data = (topics::last_reply_time.eq(&now), topics::reply_count.eq(topics::reply_count + 1));
                let to_topic_check = diesel::update(to_topic).set(update_data).execute(conn)?;
                if to_topic_check == 0 { return Err(ServiceError::NotFound); }

                if let Some(pid) = new_post.post_id {
                    let to_post = posts::table.filter(posts::id.eq(&pid).and(posts::topic_id.eq(&new_post.topic_id)));
                    let update_data = (posts::last_reply_time.eq(&now), posts::reply_count.eq(posts::reply_count + 1));
                    let to_post_check = diesel::update(to_post).set(update_data).execute(conn)?;
                    if to_post_check == 0 { new_post.post_id = None }
                }

                diesel::insert_into(posts::table).values(&new_post).execute(conn)?;
                Ok(PostQueryResult::AddedPost)
            }

            PostQuery::EditPost(new_post) => {
                match new_post.post_id {
                    Some(pid) => {
                        let old_post = posts::table.filter(posts::id.eq(&pid).and(posts::user_id.eq(&new_post.user_id)));
                        let update_data = posts::post_content.eq(&new_post.post_content);

                        diesel::update(old_post).set(update_data).execute(conn)?;
                        Ok(PostQueryResult::AddedPost)
                    }
                    None => Err(ServiceError::BadRequestGeneral)
                }
            }
        }
    }
}
