use actix::Handler;
use diesel::prelude::*;

use crate::model::errors::ServiceError;
use crate::model::{topic::*, post::Post, db::DbExecutor};
use crate::schema::{topics::dsl::*, posts};

impl Handler<TopicQuery> for DbExecutor {
    type Result = Result<TopicQueryResult, ServiceError>;

    fn handle(&mut self, message: TopicQuery, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();
        match message {
            TopicQuery::GetTopic(topic_id, page) => {
                let offset = (page - 1) * 50;
                let topic = topics.find(&topic_id).get_result::<Topic>(conn)?;
                let topic_posts = Post::belonging_to(&topic)
                    .order(posts::id.asc())
                    .limit(50)
                    .offset(offset)
                    .load::<Post>(conn)?;
                Ok(TopicQueryResult::GotTopic(topic))
//                match topics.find(&topic_id).first::<Topic>(conn) {
//
//                    Ok(topic) => Ok(TopicQueryResult::GotTopic(topic)),
//                    Err(_) => {
//                        Err(ServiceError::InternalServerError)
//                    }
            }

            TopicQuery::AddTopic(new_topic) => {
                diesel::insert_into(topics)
                    .values(&new_topic)
                    .execute(conn)?;
                Ok(TopicQueryResult::AddedTopic)
            }

            TopicQuery::UpdateTopic(topic_update_request) => {
                let tid = topic_update_request.id.unwrap_or(-1);
                let topic_old = topics.find(&tid).first::<Topic>(conn)?;

                if let Some(user_id_check) = topic_update_request.user_id {
                    if user_id_check != topic_old.user_id {
                        return Err(ServiceError::Unauthorized);
                    }
                }

                match topic_update_request.update_topic_data(topic_old) {
                    Ok(topic_new) => {
                        let updated_topic =
                            diesel::update(
                                topics.filter(id.eq(&tid)))
                                .set((
                                    category_id.eq(&topic_new.category_id),
                                    title.eq(&topic_new.title),
                                    body.eq(&topic_new.body),
                                    thumbnail.eq(&topic_new.thumbnail),
                                    last_reply_time.eq(&topic_new.last_reply_time),
                                    is_locked.eq(&topic_new.is_locked),
                                    updated_at.eq(&diesel::dsl::now)))
                                .get_result(conn)?;
                        Ok(TopicQueryResult::GotTopic(updated_topic))
                    }
                    Err(_) => Err(ServiceError::InternalServerError)
                }
            }
        }
    }
}
