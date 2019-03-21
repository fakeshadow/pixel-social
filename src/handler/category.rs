use actix::Handler;
use diesel::prelude::*;

use crate::model::errors::ServiceError;
use crate::model::{topic::*, category::*, db::DbExecutor};
use crate::schema::{topics::dsl::*, categories::dsl::*};

impl Handler<CategoryQuery> for DbExecutor {
    type Result = Result<CategoryQueryResult, ServiceError>;

    fn handle(&mut self, message: CategoryQuery, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();
        match message {
            CategoryQuery::GetAllCategories => {
                let categories_data = categories.load::<Category>(conn)?;
                Ok(CategoryQueryResult::GotCategories(categories_data))
            }

            CategoryQuery::GetPopular(page) => {
                let topics_data = topics.order(&updated_at.desc())
                    .limit(50).load::<Topic>(conn)?;
                Ok(CategoryQueryResult::GotTopics(topics_data))
            }

            CategoryQuery::GetCategory(category_request) => {
                let topics_data = topics.filter(&category_id.eq(category_request.categories[0])).order(&updated_at.desc()).limit(50).load::<Topic>(conn)?;
                Ok(CategoryQueryResult::GotTopics(topics_data))
            }

            _ => Ok(CategoryQueryResult::AddedCategory)
        }
    }
}