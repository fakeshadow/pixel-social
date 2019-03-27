use std::collections::HashSet;

use actix::Handler;
use diesel::prelude::*;

use crate::model::errors::ServiceError;
use crate::model::{topic::*, category::*, user::SlimmerUser, db::DbExecutor};
use crate::schema::{topics, users, categories};

const LIMIT: i64 = 20;
const VEC_CAPACITY: usize = 20;

impl Handler<CategoryQuery> for DbExecutor {
    type Result = Result<CategoryQueryResult, ServiceError>;

    fn handle(&mut self, message: CategoryQuery, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();

        match message {
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
                let page = category_request.page.unwrap_or(1);
                let offset = (page - 1) * LIMIT;
                let category_vec = category_request.categories.unwrap_or(vec![1]);

                let _topics: Vec<Topic> = topics::table
                    .filter(topics::category_id.eq_any(&category_vec))
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
                let modify_type = category_request.modify_type.unwrap();

                match category_request.category_data {
                    Some(category_data) => {
                        let target_category_id = category_request.category_id.unwrap_or(0);
                        let exist_category = categories::table
                            .filter(categories::name.eq(&category_data.name))
                            .execute(conn)?;

                        if modify_type == 0 && exist_category == 0 {
                            diesel::insert_into(categories::table)
                                .values(&category_data)
                                .execute(conn)?;
                        } else if modify_type == 1 && exist_category > 0 {
                            let update_field = (
                                categories::name.eq(&category_data.name),
                                categories::theme.eq(&category_data.theme));

                            diesel::update(categories::table
                                .filter(categories::id.eq(&target_category_id)))
                                .set(update_field)
                                .execute(conn)?;
                        } else if modify_type == 2 && exist_category > 0 {
                            diesel::delete(categories::table.find(&target_category_id))
                                .execute(conn)?;
                        } else {
                            return Err(ServiceError::BadRequestGeneral);
                        }

                        Ok(CategoryQueryResult::ModifiedCategory)
                    }
                    None => Err(ServiceError::BadRequestGeneral)
                }
            }
        }
    }
}

fn join_topics_users(topics: Vec<Topic>, conn: &PgConnection) -> Result<CategoryQueryResult, ServiceError> {
    if topics.len() == 0 { return Ok(CategoryQueryResult::GotTopics(vec![])); };

    let select_user_columns = (
        users::id,
        users::username,
        users::avatar_url,
        users::updated_at);

    let mut result: Vec<&i32> = Vec::with_capacity(VEC_CAPACITY);
    let mut hash_set = HashSet::with_capacity(VEC_CAPACITY);

    for topic in topics.iter() {
        if !hash_set.contains(&topic.user_id) {
            result.push(&topic.user_id);
            hash_set.insert(&topic.user_id);
        }
    };

    let users: Vec<SlimmerUser> = users::table
        .filter(users::id.eq_any(&result))
        .select(&select_user_columns)
        .load::<SlimmerUser>(conn)?;

    Ok(CategoryQueryResult::GotTopics(
        topics
            .into_iter()
            .map(|topic| topic.attach_user(&users))
            .collect()
    ))
}