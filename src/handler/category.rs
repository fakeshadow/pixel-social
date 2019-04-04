use actix_web::web;
use diesel::prelude::*;

use crate::model::common::{PostgresPool, RedisPool, QueryOption};
use crate::model::errors::ServiceError;
use crate::model::{category::*, topic::*, user::SlimUser};
use crate::schema::{categories, topics, users};

const LIMIT: i64 = 20;


// async db test
use futures::future::{join_all, ok as fut_ok, Future};
pub fn category_handler_test(
    category_query: CategoryQueryTest,
    db_pool: web::Data<PostgresPool>,
) -> impl Future<Item=CategoryQueryResult, Error=ServiceError> {

    web::block(move|| {
        let conn: &PgConnection = &db_pool.get().unwrap();
        match category_query {
            CategoryQueryTest::GetCategory(category_request) => {
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
        }
    })
    .from_err()
}

// sync db query
pub fn category_handler(
    category_query: CategoryQuery,
    opt: QueryOption,
) -> Result<CategoryQueryResult, ServiceError> {
    let db_pool = opt.db_pool.unwrap();
    let conn: &PgConnection = &db_pool.get().unwrap();

    match category_query {
        CategoryQuery::GetPopular(page) => {
            let offset = (page - 1) * LIMIT;

            let _topics: Vec<Topic> = topics::table
                .order(topics::last_reply_time.desc())
                .limit(LIMIT)
                .offset(offset)
                .load::<Topic>(conn)?;

            join_topics_users(_topics, conn)
        }

        CategoryQuery::GetCategory(category_request) => {
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

        CategoryQuery::GetAllCategories => {
            let categories_data = categories::table.load::<Category>(conn)?;
            Ok(CategoryQueryResult::GotCategories(categories_data))
        }

        CategoryQuery::ModifyCategory(category_request) => {
            Ok(CategoryQueryResult::ModifiedCategory)

            //            match category_request.category_data {
            //                Some(category_data) => {
            //                    let target_category_id = category_request.category_id.unwrap_or(0);
            //                    let exist_category = categories::table
            //                        .filter(categories::name.eq(&category_data.name))
            //                        .execute(conn)?;
            //
            //                    if modify_type == 0 && exist_category == 0 {
            //                        diesel::insert_into(categories::table)
            //                            .values(&category_data)
            //                            .execute(conn)?;
            //                    } else if modify_type == 1 && exist_category > 0 {
            //                        let update_field = (
            //                            categories::name.eq(&category_data.name),
            //                            categories::theme.eq(&category_data.theme));
            //
            //                        diesel::update(categories::table
            //                            .filter(categories::id.eq(&target_category_id)))
            //                            .set(update_field)
            //                            .execute(conn)?;
            //                    } else if modify_type == 2 && exist_category > 0 {
            //                        diesel::delete(categories::table.find(&target_category_id))
            //                            .execute(conn)?;
            //                    } else {
            //                        return Err(ServiceError::BadRequestGeneral);
            //                    }
            //
            //                    Ok(CategoryQueryResult::ModifiedCategory)
            //                }
            //                None => Err(ServiceError::BadRequestGeneral)
            //            }
        }
    }
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