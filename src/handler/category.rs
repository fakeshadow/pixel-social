use futures::Future;

use actix_web::{HttpResponse, web::block};
use diesel::prelude::*;

use crate::model::{
    category::{Category, CategoryUpdateRequest, CategoryQuery},
    common::{ match_id,PostgresPool, PoolConnectionPostgres, RedisPool},
    errors::ServiceError,
};
use crate::schema::categories;

const LIMIT: i64 = 20;

impl CategoryQuery {
    pub fn into_categories(self, pool: &PostgresPool) -> impl Future<Item=Vec<Category>, Error=ServiceError> {
        let pool = pool.clone();
        block(move || match self {
            CategoryQuery::UpdateCategory(req) => update_category(&req, &pool.get()?),
            CategoryQuery::AddCategory(req) => add_category(&req, &pool.get()?),
            CategoryQuery::GetAllCategories => get_all_categories(&pool.get()?),
            _ => panic!("method not allowed")
        }).from_err()
    }
    pub fn into_category_id(self, pool: &PostgresPool) -> impl Future<Item=u32, Error=ServiceError> {
        let pool = pool.clone();
        block(move || match self {
            CategoryQuery::DeleteCategory(id) => delete_category(&id, &pool.get()?),
            _ => panic!("only category delete query can use into_delete method")
        }).from_err()
    }
}

fn get_all_categories(conn: &PoolConnectionPostgres) -> Result<Vec<Category>, ServiceError> {
    Ok(categories::table.order(categories::id.asc()).load::<Category>(conn)?)
}

fn add_category(req: &CategoryUpdateRequest, conn: &PoolConnectionPostgres)
                -> Result<Vec<Category>, ServiceError> {
    let last_cid = Ok(categories::table
        .select(categories::id).order(categories::id.desc()).limit(1).load(conn)?);

    /// thread will panic if the database failed to get last_cid
    let next_cid = match_id(last_cid);
    Ok(diesel::insert_into(categories::table)
        .values(&req.make_category(&next_cid)?)
        .get_results(conn)?)
}

fn update_category(req: &CategoryUpdateRequest, conn: &PoolConnectionPostgres)
                   -> Result<Vec<Category>, ServiceError> {
    Ok(diesel::update(categories::table
        .filter(categories::id.eq(&req.category_id.ok_or(ServiceError::BadRequest)?)))
        .set(&req.make_update()).get_results(conn)?)
}

fn delete_category(id: &u32, conn: &PoolConnectionPostgres)
                   -> Result<u32, ServiceError> {
    diesel::delete(categories::table.find(id)).execute(conn)?;
    Ok(*id)
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
