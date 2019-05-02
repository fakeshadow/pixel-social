use actix_web::{web, HttpResponse};
use diesel::prelude::*;

use crate::model::{
    errors::ServiceError,
    post::Post,
    user::User,
    topic::{Topic, TopicWithPost, TopicQuery, TopicQueryResult, TopicRequest},
    common::{PoolConnectionPostgres, QueryOption, AttachUser},
};
use crate::handler::user::get_unique_users;
use crate::schema::{categories, posts, topics};

const LIMIT: i64 = 20;

type QueryResult = Result<HttpResponse, ServiceError>;

impl TopicQuery {
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

    let offset = (page - 1) * 20;
    let topic_raw: Topic = topics::table.filter(topics::id.eq(&id)).first::<Topic>(conn)?;
    let posts_raw: Vec<Post> = posts::table.filter(posts::topic_id.eq(&id)).order(posts::id.asc()).limit(LIMIT).offset(offset).load::<Post>(conn)?;
    let users: Vec<User> = get_unique_users(&posts_raw, Some(topic_raw.user_id), &conn)?;

    let topic = topic_raw.attach_user(&users);
    let posts = posts_raw.into_iter().map(|post| post.attach_user(&users)).collect();
    let result = if page == &1 {
        TopicWithPost::new(Some(topic), Some(posts))
    } else {
        TopicWithPost::new(None, Some(posts))
    };

    Ok(TopicQueryResult::GotTopic(&result).to_response())
}

fn add_topic(req: &TopicRequest, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get().unwrap();

    // ToDo: increment category topic count instead of only checking.
    let category_check: usize = categories::table.find(&req.extract_category_id()?).execute(conn)?;
    if category_check == 0 { return Err(ServiceError::NotFound); };

    let id: u32 = opt.global_var.unwrap().lock()
        .map(|mut guarded_global_var| guarded_global_var.next_tid())
        .map_err(|_| ServiceError::InternalServerError)?;

    diesel::insert_into(topics::table).values(&req.make_topic(&id)?).execute(conn)?;
    Ok(TopicQueryResult::ModifiedTopic.to_response())
}

fn update_topic(req: &TopicRequest, opt: &QueryOption) -> QueryResult {
    let topic_self_id = req.extract_self_id()?;
    let conn = &opt.db_pool.unwrap().get().unwrap();

    match req.user_id {
        Some(_user_id) => diesel::update(topics::table
            .filter(topics::id.eq(&topic_self_id).and(topics::user_id.eq(_user_id))))
            .set(req.make_update()?).execute(conn)?,
        None => diesel::update(topics::table
            .filter(topics::id.eq(&topic_self_id)))
            .set(req.make_update()?).execute(conn)?
    };
    Ok(TopicQueryResult::ModifiedTopic.to_response())
}

pub fn get_topic_list(cid: &u32, conn: &PoolConnectionPostgres) -> Result<Vec<u32>, ServiceError> {
    let result = topics::table.select(topics::id)
        .filter(topics::category_id.eq(&cid)).order(topics::last_reply_time.desc()).load::<u32>(conn)?;
    Ok(result)
}

pub fn get_last_tid(conn: &PoolConnectionPostgres) -> Result<Vec<u32>, ServiceError> {
    Ok(topics::table.select(topics::id).order(topics::id.desc()).limit(1).load(conn)?)
}