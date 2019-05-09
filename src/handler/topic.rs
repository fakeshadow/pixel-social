use futures::Future;

use actix_web::web::{Data, block};
use diesel::prelude::*;
use chrono::NaiveDateTime;

use crate::handler::{
    category::update_category_topic_count,
    post::get_posts_by_topic_id,
    user::get_unique_users,
};
use crate::model::{
    common::{PoolConnectionPostgres, PostgresPool, GlobalGuard},
    errors::ServiceError,
    post::Post,
    category::Category,
    topic::{Topic, TopicQuery, TopicRequest},
};
use crate::schema::topics;

const LIMIT: i64 = 20;

impl TopicQuery {
    pub fn into_topic_with_post(self, pool: &PostgresPool)
                                -> impl Future<Item=(Option<Topic>, Vec<Post>), Error=ServiceError> {
        let pool = pool.clone();
        block(move || match self {
            TopicQuery::GetTopic(id, page) => get_topic(&id, &page, &pool.get()?),
            _ => panic!("Only getting topic query can use into_topic_with_post method")
        }).from_err()
    }
    pub fn into_topic_with_category(self, pool: &PostgresPool, opt: Option<Data<GlobalGuard>>)
                                    -> impl Future<Item=(Category, Topic), Error=ServiceError> {
        let pool = pool.clone();
        block(move || match self {
            TopicQuery::AddTopic(req) => add_topic(&req, &pool.get()?, opt),
            _ => panic!("Only modify topic query can use into_topic method")
        }).from_err()
    }
    pub fn into_topic(self, pool: &PostgresPool, opt: Option<Data<GlobalGuard>>)
                      -> impl Future<Item=Topic, Error=ServiceError> {
        let pool = pool.clone();
        block(move || match self {
            TopicQuery::UpdateTopic(req) => update_topic(&req, &pool.get()?),
            _ => panic!("Only modify topic query can use into_topic method")
        }).from_err()
    }
    pub fn into_topics(self, pool: &PostgresPool)
                       -> impl Future<Item=Vec<Topic>, Error=ServiceError> {
        let pool = pool.clone();
        block(move || match self {
            TopicQuery::GetTopics(ids, page) => get_topics_by_category_id(&ids, &page, &pool.get()?),
            _ => panic!("Only getting topics query can use into_topics method")
        }).from_err()
    }
}

fn get_topic(id: &u32, page: &i64, conn: &PoolConnectionPostgres)
             -> Result<(Option<Topic>, Vec<Post>), ServiceError> {
    let posts = get_posts_by_topic_id(id, (page - 1) * 20, conn)?;
    let topic = if page == &1 {
        Some(topics::table.find(&id).first::<Topic>(conn)?)
    } else { None };
    Ok((topic, posts))
}

fn add_topic(req: &TopicRequest, conn: &PoolConnectionPostgres, global: Option<Data<GlobalGuard>>)
             -> Result<(Category, Topic), ServiceError> {
    // ToDo: Test if category_id can be null or out of range
    let category = update_category_topic_count(req.extract_category_id()?, conn)?;

    let id: u32 = global.unwrap().lock()
        .map(|mut var| var.next_tid())
        .map_err(|_| ServiceError::InternalServerError)?;
    let topic = diesel::insert_into(topics::table)
        .values(&req.make_topic(&id)?)
        .get_result::<Topic>(conn)?;
    Ok((category, topic))
}

fn update_topic(req: &TopicRequest, conn: &PoolConnectionPostgres) -> Result<Topic, ServiceError> {
    let topic_id = req.extract_self_id()?;

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

pub fn update_topic_reply_count(id: &u32, now: &NaiveDateTime, conn: &PoolConnectionPostgres) -> Result<Topic, ServiceError> {
    Ok(diesel::update(topics::table
        .filter(topics::id.eq(&id)))
        .set((topics::last_reply_time.eq(&now), topics::reply_count.eq(topics::reply_count + 1)))
        .get_result(conn)?)
}

pub fn get_topics_by_category_id(ids: &Vec<u32>, offset: &i64, conn: &PoolConnectionPostgres) -> Result<Vec<Topic>, ServiceError> {
    Ok(topics::table
        .filter(topics::category_id.eq_any(ids))
        .order(topics::last_reply_time.desc()).limit(LIMIT).offset(*offset).load::<Topic>(conn)?)
}

pub fn get_topic_list(cid: &u32, conn: &PoolConnectionPostgres) -> Result<Vec<u32>, ServiceError> {
    Ok(topics::table.select(topics::id)
        .filter(topics::category_id.eq(&cid)).order(topics::last_reply_time.desc()).load::<u32>(conn)?)
}

pub fn get_last_tid(conn: &PoolConnectionPostgres) -> Result<Vec<u32>, ServiceError> {
    Ok(topics::table.select(topics::id).order(topics::id.desc()).limit(1).load(conn)?)
}