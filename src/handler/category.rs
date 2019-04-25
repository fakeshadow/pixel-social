use actix_web::web;
use diesel::prelude::*;

use crate::model::{
    user::User,
    topic::Topic,
    errors::ServiceError,
    category::{Category, CategoryQuery, CategoryQueryResult},
    common::{PostgresPool, RedisPool, QueryOption, AttachUser, get_unique_id, match_id},
};
use crate::schema::{categories, topics, users};
use crate::model::category::{CategoryRequest, CategoryUpdateRequest};

const LIMIT: i64 = 20;

type QueryResult = Result<CategoryQueryResult, ServiceError>;

impl<'a> CategoryQuery<'a> {
    pub fn handle_query(self, opt: &QueryOption) -> QueryResult {
        let conn: &PgConnection = &opt.db_pool.unwrap().get().unwrap();
        match self {
            CategoryQuery::GetPopular(page) => get_popular(&page, &conn),
            CategoryQuery::GetCategory(category_request) => get_category(&category_request, &conn),
            CategoryQuery::GetAllCategories => get_all_categories(&conn),
            CategoryQuery::AddCategory(category_request) => add_category(&category_request, &conn),
            CategoryQuery::UpdateCategory(category_request) => update_category(&category_request, &conn),
            CategoryQuery::DeleteCategory(category_id) => delete_category(&category_id, &conn)
        }
    }
}

fn get_popular(page: &i64, conn: &PgConnection) -> QueryResult {
    let offset = (page - 1) * LIMIT;
    let _topics: Vec<Topic> = topics::table.order(topics::last_reply_time.desc()).limit(LIMIT).offset(offset).load::<Topic>(conn)?;

    join_topics_users(_topics, conn)
}

fn get_category(req: &CategoryRequest, conn: &PgConnection) -> QueryResult {
    let offset = (req.page - 1) * LIMIT;
    let _topics: Vec<Topic> = topics::table
        .filter(topics::category_id.eq_any(req.categories))
        .order(topics::last_reply_time.desc()).limit(LIMIT).offset(offset).load::<Topic>(conn)?;

    join_topics_users(_topics, conn)
}

fn get_all_categories(conn: &PgConnection) -> QueryResult {
    let categories_data = categories::table.load::<Category>(conn)?;
    Ok(CategoryQueryResult::GotCategories(categories_data))
}

fn add_category(req: &CategoryUpdateRequest, conn: &PgConnection) -> QueryResult {
    let last_cid = categories::table.select(categories::id).order(categories::id.desc()).limit(1).load(conn);
    /// thread will panic if the database failed to get last_cid
    let next_cid = match_id(last_cid);

    diesel::insert_into(categories::table).values(&req.make_category(&next_cid)?).execute(conn)?;
    Ok(CategoryQueryResult::UpdatedCategory)
}

fn update_category(req: &CategoryUpdateRequest, conn: &PgConnection) -> QueryResult {
    diesel::update(categories::table
        .filter(categories::id.eq(&req.category_id.ok_or(ServiceError::BadRequestGeneral)?)))
        .set(&req.insert()).execute(conn)?;
    Ok(CategoryQueryResult::UpdatedCategory)
}

fn delete_category(category_id: &u32, conn: &PgConnection) -> QueryResult {
    diesel::delete(categories::table.find(category_id)).execute(conn)?;
    Ok(CategoryQueryResult::UpdatedCategory)
}

fn join_topics_users(
    topics: Vec<Topic>,
    conn: &PgConnection,
) -> Result<CategoryQueryResult, ServiceError> {
    if topics.len() == 0 { return Ok(CategoryQueryResult::GotTopics(vec![])); };

    let user_ids = get_unique_id(&topics, None);
    let users: Vec<User> = users::table.filter(users::id.eq_any(&user_ids)).load::<User>(conn)?;

    Ok(CategoryQueryResult::GotTopics(topics.into_iter().map(|topic| topic.attach_from_raw(&users)).collect()))
}
