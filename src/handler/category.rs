use futures::{
    future::{err as ft_err, Either},
    Future,
};
use std::fmt::Write;

use crate::handler::{cache::CacheService, db::DatabaseService};
use crate::model::{
    category::{Category, CategoryRequest},
    common::GlobalVars,
    errors::ResError,
};

impl DatabaseService {
    pub fn get_categories_all(&self) -> impl Future<Item = Vec<Category>, Error = ResError> {
        use crate::handler::db::SimpleQuery;
        self.simple_query_multi_trait("SELECT * FROM categories", Vec::new())
    }

    pub fn update_category(
        &self,
        c: CategoryRequest,
    ) -> impl Future<Item = Category, Error = ResError> {
        let mut query = String::new();
        query.push_str("UPDATE categories SET");
        if let Some(s) = c.thumbnail {
            let _ = write!(&mut query, " thumbnail='{}',", s);
        }
        if let Some(s) = c.name {
            let _ = write!(&mut query, " name='{}',", s);
        }
        if query.ends_with(',') {
            query.remove(query.len() - 1);
            let _ = write!(&mut query, " WHERE id='{}' RETURNING *", c.id.unwrap());
        } else {
            return Either::A(ft_err(ResError::BadRequest));
        };

        use crate::handler::db::SimpleQuery;
        Either::B(self.simple_query_one_trait(query.as_str()))
    }

    pub fn remove_category(&self, cid: u32) -> impl Future<Item = (), Error = ResError> {
        let query = format!("DELETE FROM categories WHERE id={}", cid);

        use crate::handler::db::SimpleQuery;
        self.simple_query_row_trait(query.as_str()).map(|_| ())
    }

    pub fn add_category(
        &self,
        c: CategoryRequest,
        g: &GlobalVars,
    ) -> impl Future<Item = Category, Error = ResError> {
        use crate::handler::db::SimpleQuery;

        let cid = match g.lock() {
            Ok(mut g) => g.next_cid(),
            Err(_) => return Either::A(ft_err(ResError::InternalServerError)),
        };

        let query = format!(
            "
                    INSERT INTO categories
                    (id, name, thumbnail)
                    VALUES ('{}', '{}', '{}')
                    RETURNING *",
            cid,
            c.name.unwrap(),
            c.thumbnail.unwrap()
        );

        Either::B(self.simple_query_one_trait(query.as_str()))
    }
}

impl CacheService {
    pub fn get_categories_all(&self) -> impl Future<Item = Vec<Category>, Error = ResError> {
        use crate::handler::cache::CategoriesFromCache;
        self.categories_from_cache()
    }
}
