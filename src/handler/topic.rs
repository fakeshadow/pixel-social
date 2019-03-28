use actix::Handler;
use diesel::prelude::*;

use crate::model::errors::ServiceError;
use crate::model::{topic::*, post::Post, user::SlimUser, db::DbExecutor};
use crate::schema::{topics, posts, users};

const LIMIT: i64 = 20;

impl Handler<TopicQuery> for DbExecutor {
    type Result = Result<TopicQueryResult, ServiceError>;

    fn handle(&mut self, message: TopicQuery, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();

        match message {
            TopicQuery::GetTopic(topic_id, page) => {
                let offset = (page - 1) * 20;

                let _topic: Topic = topics::table
                    .filter(topics::id.eq(&topic_id))
                    .first::<Topic>(conn)?;

                let _posts: Vec<Post> = posts::table
                    .filter(posts::topic_id.eq(&_topic.id))
                    .order(posts::id.asc())
                    .limit(LIMIT)
                    .offset(offset)
                    .load::<Post>(conn)?;

                join_topics_users(_topic, _posts, conn, &page)
            }

            TopicQuery::AddTopic(new_topic) => {
                let tid = new_topic.category_id;

                let category_check: usize = topics::table.find(&tid).execute(conn)?;
                if category_check == 0 { return Err(ServiceError::NotFound); };

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

fn join_topics_users(topic: Topic, posts: Vec<Post>, conn: &PgConnection, page: &i64) -> Result<TopicQueryResult, ServiceError> {
    let select_user_columns = (
        users::id,
        users::username,
        users::email,
        users::avatar_url,
        users::signature,
        users::created_at,
        users::updated_at);

    // use to bring the trait to scope
    use crate::model::common::MatchUser;
    let result = Post::get_unique_id(&posts, Some(&topic.user_id));

    let users: Vec<SlimUser> = users::table
        .filter(users::id.eq_any(&result))
        .select(&select_user_columns)
        .load::<SlimUser>(conn)?;

    let posts = posts
        .into_iter()
        .map(|post| post.attach_user(&users))
        .collect();
    let result = if page == &1 {
        TopicResponseSlim {
            topic: Some(topic.attach_user(&users)),
            posts: Some(posts),
        }
    } else {
        TopicResponseSlim {
            topic: None,
            posts: Some(posts),
        }
    };

    Ok(TopicQueryResult::GotTopicSlim(result))
}