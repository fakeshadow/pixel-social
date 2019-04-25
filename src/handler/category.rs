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

fn get_category(category_request: &CategoryRequest, conn: &PgConnection) -> QueryResult {
    let page = category_request.page;
    let offset = (page - 1) * LIMIT;
    let categories_vec = category_request.categories;

    let _topics: Vec<Topic> = topics::table
        .filter(topics::category_id.eq_any(categories_vec))
        .order(topics::last_reply_time.desc())
        .limit(LIMIT).offset(offset).load::<Topic>(conn)?;

    join_topics_users(_topics, conn)
}

fn get_all_categories(conn: &PgConnection) -> QueryResult {
    let categories_data = categories::table.load::<Category>(conn)?;
    Ok(CategoryQueryResult::GotCategories(categories_data))
}

fn add_category(category_request: &CategoryUpdateRequest, conn: &PgConnection) -> QueryResult {
    let last_cid = categories::table.select(categories::id)
        .order(categories::id.desc()).limit(1).load(conn);
    let next_cid = match_id(last_cid);
    let new_category = category_request.make_category(&next_cid)?;

    diesel::insert_into(categories::table).values(&new_category).execute(conn)?;
    Ok(CategoryQueryResult::UpdatedCategory)
}

fn update_category(category_request: &CategoryUpdateRequest, conn: &PgConnection) -> QueryResult {
    let target_category_id = category_request.category_id.ok_or(ServiceError::BadRequestGeneral)?;
    let category_old_filter = categories::table.filter(categories::id.eq(&target_category_id));

    diesel::update(category_old_filter).set(&category_request.insert()).execute(conn)?;
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
    let result = get_unique_id(&topics, None);

    let users: Vec<User> = users::table.filter(users::id.eq_any(&result)).load::<User>(conn)?;

    Ok(CategoryQueryResult::GotTopics(
        topics.into_iter().map(|topic| topic.attach_from_raw(&users)).collect(),
    ))
}
