use actix::Handler;
use diesel::prelude::*;

use crate::model::errors::ServiceError;
use crate::model::{topic::*, db::DbExecutor};
use crate::schema::topics::dsl::*;

impl  Handler<TopicQuery> for DbExecutor {
    type Result = Result<TopicQueryResult, ServiceError>;

    fn handle(&mut self, message: TopicQuery, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();
        match message {
            TopicQuery::GetTopic(topic_id) => {
                match topics.find(&topic_id).first::<Topic>(conn) {
                    Ok(topic) => Ok(TopicQueryResult::GotTopic(topic)),
                    Err(_) => {
                        Err(ServiceError::InternalServerError)
                    }
                }
            }
            TopicQuery::AddTopic(new_topic) => {
                diesel::insert_into(topics)
                    .values(&new_topic)
                    .execute(conn)?;
                Ok(TopicQueryResult::AddedTopic)
            }
        }
    }
}
