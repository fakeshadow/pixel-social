use actix_web::{web, HttpResponse};
use diesel::prelude::*;
use chrono::NaiveDateTime;

use crate::schema::topics;
use crate::model::{
    errors::ServiceError,
    topic::{Topic, TopicWithPost, TopicQuery, TopicRequest},
    common::{PoolConnectionPostgres, QueryOption, AttachUser, Response},
};
use crate::handler::{
    cache::UpdateCache,
    user::get_unique_users,
    post::get_posts_by_topic_id,
    category::update_category_topic_count
};

const LIMIT: i64 = 20;

type QueryResult = Result<HttpResponse, ServiceError>;

impl<'a> TopicQuery<'a> {
    pub fn handle_query(self, opt: &QueryOption) -> QueryResult {
        match self {
            TopicQuery::GetTopic(topic_id, page) => get_topic(&topic_id, &page, &opt),
            TopicQuery::AddTopic(new_topic_request) => add_topic(&new_topic_request, &opt),
            TopicQuery::UpdateTopic(topic_request) => update_topic(&topic_request, &opt)
        }
    }
}

fn get_topic(id: &u32, page: &i64, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get().unwrap();

    let posts = get_posts_by_topic_id(id, (page - 1) * 20, conn)?;
    let _ignore = UpdateCache::GotPosts(&posts).handle_update(&opt.cache_pool);

    let (topic, users) = if page == &1 {
        let topic: Topic = topics::table.find(&id).first::<Topic>(conn)?;
        let _ignore = UpdateCache::GotTopic(&topic).handle_update(&opt.cache_pool);
        let users = get_unique_users(&posts, Some(topic.user_id), &conn)?;
        (Some(topic), users)
    } else {
        (None, get_unique_users(&posts, None, &conn)?)
    };

    Ok(HttpResponse::Ok().json(&TopicWithPost::new(
        topic.as_ref().map(|t| t.attach_user(&users)),
        Some(posts.iter().map(|post| post.attach_user(&users)).collect()))))
}

fn add_topic(req: &TopicRequest, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get().unwrap();

    // ToDo: Test if category_id can be null or out of range
    let category = update_category_topic_count(req.extract_category_id()?, conn)?;

    let id: u32 = opt.global_var.unwrap().lock()
        .map(|mut guarded_global_var| guarded_global_var.next_tid())
        .map_err(|_| ServiceError::InternalServerError)?;
    let topic = diesel::insert_into(topics::table).values(&req.make_topic(&id)?).get_result::<Topic>(conn)?;
    let _ignore = UpdateCache::AddedTopic(&topic, &category).handle_update(&opt.cache_pool);

    Ok(Response::ModifiedTopic.to_res())
}

fn update_topic(req: &TopicRequest, opt: &QueryOption) -> QueryResult {
    let topic_id = req.extract_self_id()?;
    let conn = &opt.db_pool.unwrap().get().unwrap();

    let topic = match req.user_id {
        Some(_user_id) => diesel::update(topics::table
            .filter(topics::id.eq(&topic_id).and(topics::user_id.eq(_user_id))))
            .set(req.make_update()?).get_result(conn)?,
        None => diesel::update(topics::table
            .find(&topic_id))
            .set(req.make_update()?).get_result(conn)?
    };
    let _ignore = UpdateCache::GotTopic(&topic).handle_update(&opt.cache_pool);

    Ok(Response::ModifiedTopic.to_res())
}

pub fn update_topic_reply_count(id: &u32, now: &NaiveDateTime, conn: &PoolConnectionPostgres) -> Result<Topic, ServiceError> {
    Ok(diesel::update(topics::table
        .filter(topics::id.eq(&id)))
        .set((topics::last_reply_time.eq(&now), topics::reply_count.eq(topics::reply_count + 1)))
        .get_result(conn)?)
}

pub fn get_topic_list(cid: &u32, conn: &PoolConnectionPostgres) -> Result<Vec<u32>, ServiceError> {
    Ok(topics::table.select(topics::id)
        .filter(topics::category_id.eq(&cid)).order(topics::last_reply_time.desc()).load::<u32>(conn)?)
}

pub fn get_last_tid(conn: &PoolConnectionPostgres) -> Result<Vec<u32>, ServiceError> {
    Ok(topics::table.select(topics::id).order(topics::id.desc()).limit(1).load(conn)?)
}