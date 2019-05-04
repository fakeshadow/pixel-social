use actix_web::{web, HttpResponse};
use diesel::prelude::*;

use crate::model::{
    errors::ServiceError,
    user::{User, ToUserRef},
    topic::{Topic, TopicWithUser},
    category::{Category, CategoryQuery, CategoryRequest, CategoryUpdateRequest},
    common::{Response, PoolConnectionPostgres, RedisPool, QueryOption, GetUserId, AttachUser, get_unique_id, match_id},
};
use crate::handler::{
    user::get_unique_users,
    cache::UpdateCache,
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
    let conn = &opt.db_pool.unwrap().get()?;

    let offset = (page - 1) * LIMIT;
    let topics: Vec<Topic> = topics::table.order(topics::last_reply_time.desc()).limit(LIMIT).offset(offset).load::<Topic>(conn)?;
    let users = get_unique_users(&topics, None, &conn)?;

    let _ignore = UpdateCache::GotTopics(&topics).handle_update(&opt.cache_pool);

    Ok(HttpResponse::Ok().json(&topics.iter().map(|topic| topic.attach_user(&users)).collect::<Vec<TopicWithUser>>()))
}

fn get_category(req: &CategoryRequest, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get()?;

    let offset = (req.page - 1) * LIMIT;
    let topics: Vec<Topic> = topics::table
        .filter(topics::category_id.eq_any(req.categories))
        .order(topics::last_reply_time.desc()).limit(LIMIT).offset(offset).load::<Topic>(conn)?;
    let users = get_unique_users(&topics, None, &conn)?;

    let _ignore = UpdateCache::GotTopics(&topics).handle_update(&opt.cache_pool);
    Ok(HttpResponse::Ok().json(&topics.iter().map(|topic| topic.attach_user(&users)).collect::<Vec<TopicWithUser>>()))
}

fn get_all_categories(opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get()?;
    let categories = categories::table.order(categories::id.asc()).load::<Category>(conn)?;

    let _ignore = UpdateCache::GotCategories(&categories).handle_update(&opt.cache_pool);
    Ok(HttpResponse::Ok().json(&categories))
}

fn add_category(req: &CategoryUpdateRequest, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get().unwrap();

    let last_cid = Ok(categories::table
        .select(categories::id).order(categories::id.desc()).limit(1).load(conn)?);

    /// thread will panic if the database failed to get last_cid
    let next_cid = match_id(last_cid);
    let category: Category = diesel::insert_into(categories::table).values(&req.make_category(&next_cid)?).get_result(conn)?;

    let _ignore = UpdateCache::GotCategories(&vec![category]).handle_update(&opt.cache_pool);

    Ok(Response::UpdatedCategory.to_res())
}

fn update_category(req: &CategoryUpdateRequest, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get()?;

    let category: Category = diesel::update(categories::table
        .filter(categories::id.eq(&req.category_id.ok_or(ServiceError::BadRequestGeneral)?)))
        .set(&req.insert()).get_result(conn)?;

    let _ignore = UpdateCache::GotCategories(&vec![category]).handle_update(&opt.cache_pool);

    Ok(Response::UpdatedCategory.to_res())
}

fn delete_category(id: &u32, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get()?;
    diesel::delete(categories::table.find(id)).execute(conn)?;
    let _ignore = UpdateCache::DeleteCategory(id).handle_update(&opt.cache_pool);
    Ok(Response::UpdatedCategory.to_res())
}


//helper functions

pub fn update_category_post_count(id: &u32, conn: &PoolConnectionPostgres) -> Result<Category, ServiceError> {
    Ok(diesel::update(categories::table.find(id))
        .set(categories::post_count.eq(categories::post_count + 1)).get_result(conn)?)
}

pub fn update_category_topic_count(id: &u32, conn: &PoolConnectionPostgres) -> Result<Category, ServiceError> {
    Ok(diesel::update(categories::table.find(id))
        .set(categories::topic_count.eq(categories::topic_count + 1)).get_result(conn)?)
}

pub fn update_category_sub_count(id: &u32, conn: &PoolConnectionPostgres) -> Result<Category, ServiceError> {
    Ok(diesel::update(categories::table.find(id))
        .set(categories::subscriber_count.eq(categories::subscriber_count + 1)).get_result(conn)?)
}

pub fn load_all_categories(conn: &PoolConnectionPostgres) -> Result<Vec<Category>, ServiceError> {
    // ToDo: update category data on startup
    Ok(categories::table.order(categories::id.asc()).load::<Category>(conn)?)
}
