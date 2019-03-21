use actix::Handler;
use diesel::prelude::*;

use crate::model::errors::ServiceError;
use crate::model::{topic::*, category::*, db::DbExecutor};
use crate::schema::{topics::dsl::*, categories::dsl as category_table};

impl Handler<CategoryQuery> for DbExecutor {
    type Result = Result<CategoryQueryResult, ServiceError>;

    fn handle(&mut self, message: CategoryQuery, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();
        match message {
            CategoryQuery::GetAllCategories => {
                let categories_data = category_table::categories.load::<Category>(conn)?;
                Ok(CategoryQueryResult::GotCategories(categories_data))
            }

            CategoryQuery::GetPopular(page) => {
                let offset = (page as i64 - 1) * 50;
                let topics_data = topics
                    .order(&updated_at.desc())
                    .limit(50)
                    .offset(offset)
                    .load::<Topic>(conn)?;
                Ok(CategoryQueryResult::GotTopics(topics_data))
            }

            CategoryQuery::GetCategory(category_request) => {
                let page = category_request.page.unwrap_or(1);
                let offset = (page as i64 - 1) * 50;
                let category_vec = category_request.categories.unwrap_or(vec![1]);
                let topics_data = topics
                    .filter(&category_id.eq_any(&category_vec))
                    .order(&updated_at.desc())
                    .limit(50)
                    .offset(offset)
                    .load::<Topic>(conn)?;
                Ok(CategoryQueryResult::GotTopics(topics_data))
            }

            CategoryQuery::ModifyCategory(category_request) => {
                let modify_type = category_request.modify_type.unwrap();

                // add category check here

                match category_request.category_data {
                    Some(category_data) => {
                        if modify_type == 0 {
                            diesel::insert_into(category_table::categories).values(&category_data).execute(conn)?;
                            Ok(CategoryQueryResult::ModifiedCategory)
                        } else if modify_type == 1 {
                            let target_category_id = category_request.category_id.unwrap_or(0);
                            diesel::update(category_table::categories
                                .filter(category_table::id.eq(&target_category_id)))
                                .set((category_table::name.eq(&category_data.name), category_table::theme.eq(&category_data.theme)))
                                .execute(conn)?;
                            Ok(CategoryQueryResult::ModifiedCategory)
                        } else if modify_type == 2 {
                            let target_category_id = category_request.category_id.unwrap_or(0);
                            diesel::delete(category_table::categories.find(&target_category_id)).execute(conn)?;
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