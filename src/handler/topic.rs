use actix::Handler;
use diesel::prelude::*;

use crate::model::errors::ServiceError;
use crate::model::{topic::*, post::Post, user::SlimUser, db::DbExecutor};
use crate::schema::{topics, posts, users};

impl Handler<TopicQuery> for DbExecutor {
    type Result = Result<TopicQueryResult, ServiceError>;

    fn handle(&mut self, message: TopicQuery, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();
        match message {
            TopicQuery::GetTopic(topic_id, page) => {
                let limit = if page == 1 { 49 as i64 } else { 50 as i64 };
                let offset = (page - 1) * 50;

                let select_user_columns = (
                    users::id,
                    users::username,
                    users::email,
                    users::avatar_url,
                    users::signature,
                    users::created_at,
                    users::updated_at);

                let (topic, topic_user) = topics::table
                    .filter(topics::id.eq(&topic_id))
                    .inner_join(users::table)
                    .select((topics::all_columns, select_user_columns))
                    .first::<(Topic, SlimUser)>(conn)?;

                let posts_with_user: Vec<(Post, SlimUser)> = Post::belonging_to(&topic)
                    .inner_join(users::table)
                    .select((posts::all_columns, select_user_columns))
                    .order(posts::id.asc())
                    .limit(limit)
                    .offset(offset)
                    .load::<(Post, SlimUser)>(conn)?;

                let result = if page == 1 {
                    TopicResponse {
                        topic: Some(topic.attach_user(topic_user)),
                        posts: Some(posts_with_user.into_iter().map(|(post, user)| post.attach_user(user)).collect())
                    }
                } else {
                    TopicResponse {
                        topic: None,
                        posts: Some(posts_with_user.into_iter().map(|(post, user)| post.attach_user(user)).collect())
                    }
                };
                Ok(TopicQueryResult::GotTopic(result))
            }

            TopicQuery::AddTopic(new_topic) => {
                diesel::insert_into(topics::table)
                    .values(&new_topic)
                    .execute(conn)?;
                Ok(TopicQueryResult::AddedTopic)
            }

            TopicQuery::UpdateTopic(topic_update_request) => {
                let tid = topic_update_request.id.unwrap_or(-1);
                let topic_old = topics::table.find(&tid).first::<Topic>(conn)?;

                if let Some(user_id_check) = topic_update_request.user_id {
                    if user_id_check != topic_old.user_id {
                        return Err(ServiceError::Unauthorized);
                    }
                }

                match topic_update_request.update_topic_data(topic_old) {
                    Ok(topic_new) => {
                        diesel::update(
                            topics::table.filter(topics::id.eq(&tid)))
                            .set((
                                topics::category_id.eq(&topic_new.category_id),
                                topics::title.eq(&topic_new.title),
                                topics::body.eq(&topic_new.body),
                                topics::thumbnail.eq(&topic_new.thumbnail),
                                topics::last_reply_time.eq(&topic_new.last_reply_time),
                                topics::is_locked.eq(&topic_new.is_locked),
                                topics::updated_at.eq(&diesel::dsl::now)))
                            .execute(conn)?;
                        Ok(TopicQueryResult::AddedTopic)
                    }
                    Err(_) => Err(ServiceError::InternalServerError)
                }
            }
        }
    }
}
