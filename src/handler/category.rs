use std::fmt::Write;
use futures::future::err as ft_err;

use actix::prelude::*;

use crate::model::{
    actors::DatabaseService,
    category::{Category, CategoryUpdateRequest},
    errors::ServiceError,
};
use crate::handler::db::{simple_query, category_from_msg, get_all_categories};


const LIMIT: i64 = 20;

pub struct GetCategories;
pub enum ModifyCategory {
    Add(CategoryUpdateRequest),
    Update(CategoryUpdateRequest),
}

impl Message for ModifyCategory {
    type Result = Result<Vec<Category>, ServiceError>;
}

impl Message for GetCategories {
    type Result = Result<Vec<Category>, ServiceError>;
}

impl Handler<GetCategories> for DatabaseService {
    type Result = ResponseFuture<Vec<Category>, ServiceError>;

    fn handle(&mut self, _: GetCategories, _: &mut Self::Context) -> Self::Result {
        let categories = Vec::new();
        Box::new(get_all_categories(
            self.db.as_mut().unwrap(),
            self.categories.as_ref().unwrap(),
            categories))
    }
}
impl Handler<ModifyCategory> for DatabaseService {
    type Result = ResponseFuture<Vec<Category>, ServiceError>;

    fn handle(&mut self, msg: ModifyCategory, _: &mut Self::Context) -> Self::Result {
        let query = match msg {
            ModifyCategory::Add(req) => {
                let c = match req.make_category(&1) {
                    Ok(c) => c,
                    Err(e) => return Box::new(ft_err(e))
                };

                format!("INSERT INTO categories
                (id, name, thumbnail)
                VALUES ('{}', '{}', '{}')
                RETURNING *", c.id, c.name, c.thumbnail)
            }
            ModifyCategory::Update(req) => {
                let c = match req.make_update() {
                    Ok(c) => c,
                    Err(e) => return Box::new(ft_err(e))
                };

                let mut query = String::new();
                query.push_str("UPDATE categories SET");
                if let Some(s) = c.thumbnail {
                    let _ = write!(&mut query, " thumbnail='{}',", s);
                }
                if let Some(s) = c.name {
                    let _ = write!(&mut query, " name='{}',", s);
                }
                if query.ends_with(",") {
                    query.remove(query.len() - 1);
                    let _ = write!(&mut query, " WHERE id='{}' RETURNING *", c.id);
                } else {
                    return Box::new(ft_err(ServiceError::BadRequest));
                }
                query
            }
        };
        Box::new(simple_query(
            self.db.as_mut().unwrap(),
            query.as_str())
            .and_then(|msg| category_from_msg(&msg).map(|c| vec![c]))
        )
    }
}