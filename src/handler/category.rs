use actix_web::web;
use diesel::prelude::*;

use crate::model::{
    user::SlimUser,
    topic::Topic,
    errors::ServiceError,
    category::{Category, CategoryQuery, CategoryQueryResult},
    common::{PostgresPool, RedisPool, QueryOption, match_id},
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
    let _topics: Vec<Topic> = topics::table
        .order(topics::last_reply_time.desc())
        .limit(LIMIT)
        .offset(offset)
        .load::<Topic>(conn)?;

    join_topics_users(_topics, conn)
}

fn get_category(category_request: &CategoryRequest, conn: &PgConnection) -> QueryResult {
    let page = category_request.page;
    let offset = (page - 1) * LIMIT;
    let categories_vec = category_request.categories;

    let _topics: Vec<Topic> = topics::table
        .filter(topics::category_id.eq_any(categories_vec))
        .order(topics::last_reply_time.desc())
        .limit(LIMIT)
        .offset(offset)
        .load::<Topic>(conn)?;

    join_topics_users(_topics, conn)
}

fn get_all_categories(conn: &PgConnection) -> QueryResult {
    let categories_data = categories::table.load::<Category>(conn)?;
    Ok(CategoryQueryResult::GotCategories(categories_data))
}

fn add_category(category_request: &CategoryUpdateRequest, conn: &PgConnection) -> QueryResult {
    let category_name = match category_request.category_name {
        Some(name) => name,
        None => return { Err(ServiceError::BadRequestGeneral) }
    };
    let category_thumbnail = match category_request.category_thumbnail {
        Some(thumbnail) => thumbnail,
        None => return { Err(ServiceError::BadRequestGeneral) }
    };

    let last_cid = categories::table.select(categories::id)
        .order(categories::id.desc())
        .limit(1)
        .load(conn);
    let next_cid = match_id(last_cid);

    let category_data = Category::new(next_cid, &category_name, &category_thumbnail);

    diesel::insert_into(categories::table)
        .values(&category_data)
        .execute(conn)?;

    Ok(CategoryQueryResult::UpdatedCategory)
}

fn update_category(category_request: &CategoryUpdateRequest, conn: &PgConnection) -> QueryResult {
    let target_category_id = match category_request.category_id {
        Some(id) => id,
        None => return Err(ServiceError::BadRequestGeneral)
    };

    let category_old_filter = categories::table
        .filter(categories::id.eq(&target_category_id));

    diesel::update(category_old_filter).set(&category_request.insert()).execute(conn)?;

    Ok(CategoryQueryResult::UpdatedCategory)
}

fn delete_category(category_id: &u32, conn: &PgConnection) -> QueryResult {

    diesel::delete(categories::table.find(category_id))
        .execute(conn)?;
    Ok(CategoryQueryResult::UpdatedCategory)
}

fn join_topics_users(
    topics: Vec<Topic>,
    conn: &PgConnection,
) -> Result<CategoryQueryResult, ServiceError> {
    if topics.len() == 0 {
        return Ok(CategoryQueryResult::GotTopics(vec![]));
    };

    let select_user_columns = (
        users::id,
        users::username,
        users::email,
        users::avatar_url,
        users::signature,
        users::created_at,
        users::updated_at,
    );

    // use to bring the trait to scope
    use crate::model::common::MatchUser;
    let result = Topic::get_unique_id(&topics, None);

    let users: Vec<SlimUser> = users::table
        .filter(users::id.eq_any(&result))
        .select(&select_user_columns)
        .load::<SlimUser>(conn)?;

    Ok(CategoryQueryResult::GotTopics(
        topics
            .into_iter()
            .map(|topic| topic.attach_user(&users))
            .collect(),
    ))
}
