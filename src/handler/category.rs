use actix::Handler;
use diesel::prelude::*;

use crate::model::errors::ServiceError;
use crate::model::{topic::*, category::*, user::SlimmerUser, db::DbExecutor};
use crate::schema::{topics, users, categories};

impl Handler<CategoryQuery> for DbExecutor {
    type Result = Result<CategoryQueryResult, ServiceError>;

    fn handle(&mut self, message: CategoryQuery, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();
        let select_user_columns = (
            users::id,
            users::username,
            users::avatar_url,
            users::updated_at);

        match message {

            CategoryQuery::GetPopular(page) => {
                let limit = 20 as i64;
                let offset = (page - 1) * 20;
                let topics_with_user: Vec<(Topic, SlimmerUser)> = topics::table
                    .order(topics::last_reply_time.desc())
                    .limit(limit)
                    .offset(offset)
                    .inner_join(users::table)
                    .select((topics::all_columns, &select_user_columns))
                    .load::<(Topic, SlimmerUser)>(conn)?;

                Ok(CategoryQueryResult::GotTopics(topics_with_user.into_iter()
                    .map(|(topic, user)| topic.attach_slimmer_user(user))
                    .collect()))
            }

            CategoryQuery::GetCategory(category_request) => {
                let page = category_request.page.unwrap_or(1);
                let limit = 20 as i64;
                let offset = (page - 1) * 20;
                let category_vec = category_request.categories.unwrap_or(vec![1]);

                let topics_with_user: Vec<(Topic, SlimmerUser)> = topics::table
                    .filter(topics::category_id.eq_any(&category_vec))
                    .order(topics::last_reply_time.desc())
                    .limit(limit)
                    .offset(offset)
                    .inner_join(users::table)
                    .select((topics::all_columns, &select_user_columns))
                    .load::<(Topic, SlimmerUser)>(conn)?;

                Ok(CategoryQueryResult::GotTopics(topics_with_user.into_iter()
                    .map(|(topic, user)| topic.attach_slimmer_user(user))
                    .collect()))
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
                            Ok(CategoryQueryResult::ModifiedCategory)
                        } else if modify_type == 1 && exist_category > 0 {
                            diesel::update(categories::table
                                .filter(categories::id.eq(&target_category_id)))
                                .set((categories::name.eq(&category_data.name), categories::theme.eq(&category_data.theme)))
                                .execute(conn)?;
                            Ok(CategoryQueryResult::ModifiedCategory)
                        } else if modify_type == 2 && exist_category > 0 {
                            diesel::delete(categories::table.find(&target_category_id))
                                .execute(conn)?;
                            Ok(CategoryQueryResult::ModifiedCategory)
                        } else {
                            Err(ServiceError::BadRequestGeneral)
                        }
                    }
                    None => Err(ServiceError::BadRequestGeneral)
                }
            }
        }
    }
}