use std::fmt::Write;
use std::future::Future;

use futures::{
    FutureExt,
    compat::Future01CompatExt,
    TryFutureExt,
};

use crate::handler::{
    cache::CacheService,
    db::DatabaseService,
    cache_update::CacheFailedMessage
};
use crate::model::{
    category::{Category, CategoryRequest},
    common::GlobalVars,
    errors::ResError,
};

impl DatabaseService {
    pub fn get_categories_all(&self) -> impl Future<Output=Result<Vec<Category>, ResError>> {
        use crate::handler::db::SimpleQuery;
        self.simple_query_multi_trait("SELECT * FROM categories", Vec::new())
    }

    pub async fn update_category(
        &self,
        c: CategoryRequest,
    ) -> Result<Category, ResError> {
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
            return Err(ResError::BadRequest);
        };

        use crate::handler::db::SimpleQuery;
        self.simple_query_one_trait(query.as_str()).await
    }

    pub fn remove_category(&self, cid: u32) -> impl Future<Output=Result<(), ResError>> {
        let query = format!("DELETE FROM categories WHERE id={}", cid);

        use crate::handler::db::SimpleQuery;
        self.simple_query_row_trait(query.as_str()).map_ok(|_| ())
    }

    pub async fn add_category(
        &self,
        c: CategoryRequest,
        g: &GlobalVars,
    ) -> Result<Category, ResError> {
        use crate::handler::db::SimpleQuery;

        let cid = g.lock().map(|mut lock| lock.next_cid()).await;

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

        self.simple_query_one_trait(query.as_str()).await
    }
}

impl CacheService {
    pub fn get_categories_all(&self) -> impl Future<Output=Result<Vec<Category>, ResError>> {
        use crate::handler::cache::CategoriesFromCache;
        self.categories_from_cache_01().compat()
    }

    pub fn send_failed_category(&self, c: Category) { let _ = self.recipient.do_send(CacheFailedMessage::FailedCategory(c)); }
}