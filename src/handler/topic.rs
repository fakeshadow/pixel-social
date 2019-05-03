use actix_web::{web, HttpResponse};
use diesel::prelude::*;

use crate::model::{
    errors::ServiceError,
    post::Post,
    user::User,
    topic::{Topic, TopicWithPost, TopicQuery, TopicRequest},
    common::{PoolConnectionPostgres, QueryOption, AttachUser, Response},
};
use crate::handler::{
    cache::UpdateCache,
    user::get_unique_users,
};
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

// ToDo: Add category list and meta data cache update in topic handler

fn get_topic(id: &u32, page: &i64, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get().unwrap();

    let offset = (page - 1) * 20;
    let topic_raw: Topic = topics::table.filter(topics::id.eq(&id)).first::<Topic>(conn)?;
    let posts_raw: Vec<Post> = posts::table.filter(posts::topic_id.eq(&id)).order(posts::id.asc()).limit(LIMIT).offset(offset).load::<Post>(conn)?;
    let users: Vec<User> = get_unique_users(&posts_raw, Some(topic_raw.user_id), &conn)?;

    let topic_vec = vec![topic_raw];
    let _ignore = UpdateCache::TopicPostUser(Some(&topic_vec), Some(&posts_raw), None).handle_update(&opt.cache_pool);

    let posts = posts_raw.iter().map(|post| post.attach_user(&users)).collect();
    let result = if page == &1 {
        TopicWithPost::new(Some(topic_vec[0].attach_user(&users)), Some(posts))
    } else {
        TopicWithPost::new(None, Some(posts))
    };

    Ok(HttpResponse::Ok().json(&result))
}

fn add_topic(req: &TopicRequest, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get().unwrap();

    // ToDo: increment category topic count instead of only checking.
    let category_check: usize = categories::table.find(&req.extract_category_id()?).execute(conn)?;
    if category_check == 0 { return Err(ServiceError::NotFound); };

    let id: u32 = opt.global_var.unwrap().lock()
        .map(|mut guarded_global_var| guarded_global_var.next_tid())
        .map_err(|_| ServiceError::InternalServerError)?;

    let topic: Topic = diesel::insert_into(topics::table).values(&req.make_topic(&id)?).get_result(conn)?;
    // ToDo: Update Category set to cache
    let _ignore = UpdateCache::AddedTopic(&topic).handle_update(&opt.cache_pool);

    Ok(Response::ModifiedTopic.to_res())
}

fn update_topic(req: &TopicRequest, opt: &QueryOption) -> QueryResult {
    let topic_self_id = req.extract_self_id()?;
    let conn = &opt.db_pool.unwrap().get().unwrap();

    let topic: Topic = match req.user_id {
        Some(_user_id) => diesel::update(topics::table
            .filter(topics::id.eq(&topic_self_id).and(topics::user_id.eq(_user_id))))
            .set(req.make_update()?).get_result(conn)?,
        None => diesel::update(topics::table
            .filter(topics::id.eq(&topic_self_id)))
            .set(req.make_update()?).get_result(conn)?
    };
    let _ignore = UpdateCache::UpdatedTopic(&topic).handle_update(&opt.cache_pool);

    Ok(Response::ModifiedTopic.to_res())
}

pub fn get_topic_list(cid: &u32, conn: &PoolConnectionPostgres) -> Result<Vec<u32>, ServiceError> {
    Ok(topics::table.select(topics::id)
        .filter(topics::category_id.eq(&cid)).order(topics::last_reply_time.desc()).load::<u32>(conn)?)
}

pub fn get_last_tid(conn: &PoolConnectionPostgres) -> Result<Vec<u32>, ServiceError> {
    Ok(topics::table.select(topics::id).order(topics::id.desc()).limit(1).load(conn)?)
}