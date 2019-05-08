use futures::Future;

use actix_web::web::block;
use diesel::prelude::*;

use crate::handler::{
    category::update_category_topic_count,
    post::get_posts_by_topic_id,
    user::get_unique_users,
};
use crate::model::{
    common::{AttachUser, PoolConnectionPostgres, QueryOptAsync, Response},
    errors::ServiceError,
    post::Post,
    topic::{Topic, TopicQueryAsync, TopicRequest},
};
use crate::schema::topics;

const LIMIT: i64 = 20;

impl TopicQueryAsync {
    pub fn into_topic_with_post(self, opt: QueryOptAsync) -> impl Future<Item=(Option<Topic>, Vec<Post>), Error=ServiceError> {
        block(move || match self {
            TopicQueryAsync::GetTopic(id, page) => get_topic(&id, &page, opt),
            _ => panic!("Only getting topic query can use into_topic_with_post method")
        }).from_err()
    }
    pub fn into_topic(self, opt: QueryOptAsync) -> impl Future<Item=Topic, Error=ServiceError> {
        block(move || match self {
            TopicQueryAsync::AddTopic(req) => add_topic(&req, opt),
            TopicQueryAsync::UpdateTopic(req) => update_topic(&req, opt),
            _ => panic!("Only modify topic query can use into_topic method")
        }).from_err()
    }
}

fn get_topic(id: &u32, page: &i64, opt: QueryOptAsync) -> Result<(Option<Topic>, Vec<Post>), ServiceError> {
    use std::{thread::sleep, time::Duration};
    sleep(Duration::from_millis(10));

    let conn = &opt.db.unwrap().get()?;
    let posts = get_posts_by_topic_id(id, (page - 1) * 20, conn)?;
    let topic = if page == &1 {
        Some(topics::table.find(&id).first::<Topic>(conn)?)
    } else {
        None
    };
    Ok((topic, posts))
}

fn add_topic(req: &TopicRequest, opt: QueryOptAsync) -> Result<Topic, ServiceError> {
    let conn = &opt.db.unwrap().get().unwrap();

    // ToDo: Test if category_id can be null or out of range
    let category = update_category_topic_count(req.extract_category_id()?, conn)?;

    let id: u32 = opt.global.unwrap().lock()
        .map(|mut guarded_global_var| guarded_global_var.next_tid())
        .map_err(|_| ServiceError::InternalServerError)?;
    let topic = diesel::insert_into(topics::table).values(&req.make_topic(&id)?).get_result::<Topic>(conn)?;
    Ok(topic)
}

fn update_topic(req: &TopicRequest, opt: QueryOptAsync) -> Result<Topic, ServiceError> {
    let topic_id = req.extract_self_id()?;
    let conn = &opt.db.unwrap().get().unwrap();

    let topic = match req.user_id {
        Some(_user_id) => diesel::update(topics::table
            .filter(topics::id.eq(&topic_id).and(topics::user_id.eq(_user_id))))
            .set(req.make_update()?).get_result(conn)?,
        None => diesel::update(topics::table
            .find(&topic_id))
            .set(req.make_update()?).get_result(conn)?
    };
    Ok(topic)
}