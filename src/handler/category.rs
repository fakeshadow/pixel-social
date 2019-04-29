use actix_web::{web, HttpResponse};
use diesel::prelude::*;

use crate::model::{
    errors::ServiceError,
    user::{User, ToUserRef},
    topic::{Topic, TopicWithUser},
    category::{Category, CategoryQuery, CategoryQueryResult, CategoryRequest, CategoryUpdateRequest},
    common::{PoolConnectionPostgres as DbConnection, PoolConnectionRedis as CacheConnection, RedisPool, QueryOption, GetUserId, AttachUserRef, get_unique_id, match_id},
};
use crate::handler::{
    user::get_unique_users,
    cache::{UpdateCache}
};
use crate::schema::{categories, topics};

const LIMIT: i64 = 20;

type QueryResult = Result<HttpResponse, ServiceError>;

impl<'a> CategoryQuery<'a> {
    pub fn handle_query(self, opt: &QueryOption) -> QueryResult {
        match self {
            CategoryQuery::GetPopular(page) => get_popular(&page, &opt),
            CategoryQuery::GetCategory(category_request) => get_category(&category_request, &opt),
            CategoryQuery::GetAllCategories => get_all_categories(&opt),
            CategoryQuery::AddCategory(category_request) => add_category(&category_request, &opt),
            CategoryQuery::UpdateCategory(category_request) => update_category(&category_request, &opt),
            CategoryQuery::DeleteCategory(category_id) => delete_category(&category_id, &opt)
        }
    }
}

fn get_popular(page: &i64, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get().unwrap();

    let offset = (page - 1) * LIMIT;
    let topics: Vec<Topic> = topics::table.order(topics::last_reply_time.desc()).limit(LIMIT).offset(offset).load::<Topic>(conn)?;
    let users = get_unique_users(&topics, None, &conn)?;


    let topics_final = topics.iter().map(|topic| topic.to_ref().attach_user(&users)).collect();
    Ok(CategoryQueryResult::GotTopics(&topics_final).to_response())
}

fn get_category(req: &CategoryRequest, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get().unwrap();

    let offset = (req.page - 1) * LIMIT;
    let topics: Vec<Topic> = topics::table
        .filter(topics::category_id.eq_any(req.categories))
        .order(topics::last_reply_time.desc()).limit(LIMIT).offset(offset).load::<Topic>(conn)?;
    let users = get_unique_users(&topics, None, &conn)?;

    let _topic = UpdateCache::Topics(&topics).handle_update(&opt.cache_pool)?;
    let _post = UpdateCache::Users(&users).handle_update(&opt.cache_pool)?;

    let topics_final = topics.iter().map(|topic| topic.to_ref().attach_user(&users)).collect();
    Ok(CategoryQueryResult::GotTopics(&topics_final).to_response())
}

fn get_all_categories(opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get().unwrap();

    let categories_data = categories::table.load::<Category>(conn)?;
    Ok(CategoryQueryResult::GotCategories(categories_data).to_response())
}

fn add_category(req: &CategoryUpdateRequest, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get().unwrap();

    let last_cid = Ok(categories::table
        .select(categories::id).order(categories::id.desc()).limit(1).load(conn)?);

    /// thread will panic if the database failed to get last_cid
    let next_cid = match_id(last_cid);

    diesel::insert_into(categories::table).values(&req.make_category(&next_cid)?).execute(conn)?;
    Ok(CategoryQueryResult::UpdatedCategory.to_response())
}

fn update_category(req: &CategoryUpdateRequest, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get().unwrap();

    diesel::update(categories::table
        .filter(categories::id.eq(&req.category_id.ok_or(ServiceError::BadRequestGeneral)?)))
        .set(&req.insert()).execute(conn)?;
    Ok(CategoryQueryResult::UpdatedCategory.to_response())
}

fn delete_category(category_id: &u32, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get().unwrap();

    diesel::delete(categories::table.find(category_id)).execute(conn)?;
    Ok(CategoryQueryResult::UpdatedCategory.to_response())
}