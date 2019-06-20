use std::fmt::Write;
use futures::future::err as ft_err;

use actix::prelude::*;

use crate::model::{
    actors::DatabaseService,
    category::{Category, CategoryRequest},
    errors::ServiceError,
};
use crate::handler::db::{simple_query, category_from_msg, get_all_categories, single_row_from_msg, get_single_row};


const LIMIT: i64 = 20;

pub struct GetCategories;

pub struct GetLastCategoryId;

pub struct AddCategory(pub CategoryRequest);

pub struct UpdateCategory(pub CategoryRequest);

impl Message for GetLastCategoryId {
    type Result = Result<u32, ServiceError>;
}

impl Message for AddCategory {
    type Result = Result<Vec<Category>, ServiceError>;
}

impl Message for UpdateCategory {
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

impl Handler<GetLastCategoryId> for DatabaseService {
    type Result = ResponseFuture<u32, ServiceError>;

    fn handle(&mut self, _: GetLastCategoryId, _: &mut Self::Context) -> Self::Result {
        let query = "SELECT id FROM categories ORDER BY id DESC LIMIT 1";
        Box::new(get_single_row::<u32>(self.db.as_mut().unwrap(), query))
    }
}

impl Handler<AddCategory> for DatabaseService {
    type Result = ResponseFuture<Vec<Category>, ServiceError>;

    fn handle(&mut self, msg: AddCategory, _: &mut Self::Context) -> Self::Result {
        let c = msg.0;

        let query = format!("INSERT INTO categories
            (id, name, thumbnail)
            VALUES ('{}', '{}', '{}')
            RETURNING *", c.id.unwrap(), c.name.unwrap(), c.thumbnail.unwrap());

        let f = simple_query(
            self.db.as_mut().unwrap(),
            query.as_str())
            .and_then(|msg| category_from_msg(&msg).map(|c| vec![c]));

        Box::new(f)
    }
}

impl Handler<UpdateCategory> for DatabaseService {
    type Result = ResponseFuture<Vec<Category>, ServiceError>;

    fn handle(&mut self, msg: UpdateCategory, _: &mut Self::Context) -> Self::Result {
        let c = match msg.0.make_update() {
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
            let _ = write!(&mut query, " WHERE id='{}' RETURNING *", c.id.unwrap());
        } else {
            return Box::new(ft_err(ServiceError::BadRequest));
        };

        Box::new(simple_query(
            self.db.as_mut().unwrap(),
            query.as_str())
            .and_then(|msg| category_from_msg(&msg).map(|c| vec![c]))
        )
    }
}