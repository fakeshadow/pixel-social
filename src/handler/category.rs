use std::future::Future;

use futures::{compat::Future01CompatExt, FutureExt};
use futures01::Future as Future01;
use tokio_postgres::types::ToSql;

use crate::handler::{
    cache::{build_hmsets_01, CacheService, GetSharedConn, CATEGORY_U8},
    cache_update::CacheFailedMessage,
    db::{DatabaseService, GetDbClient, Query},
};
use crate::model::{
    category::{Category, CategoryRequest},
    common::GlobalVars,
    errors::ResError,
};

impl DatabaseService {
    pub async fn get_categories_all(&self) -> Result<Vec<Category>, ResError> {
        let st = self
            .get_client()
            .prepare("SELECT * FROM categories")
            .await?;
        self.query_multi_trait(&st, &[], Vec::new()).await
    }

    pub async fn update_category(&self, c: CategoryRequest) -> Result<Category, ResError> {
        let mut query = String::from("UPDATE categories SET");
        let mut params = Vec::new();
        let mut index = 1u8;

        if let Some(s) = c.thumbnail.as_ref() {
            query.push_str(" thumbnail=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &dyn ToSql);
            index += 1;
        }
        if let Some(s) = c.name.as_ref() {
            query.push_str(" name=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &dyn ToSql);
            index += 1;
        }

        if query.ends_with(',') {
            query.pop();
            query.push_str(" WHERE id=$");
            query.push_str(index.to_string().as_str());
            params.push(c.id.as_ref().unwrap() as &dyn ToSql);
        } else {
            return Err(ResError::BadRequest);
        };

        query.push_str(" RETURNING *");

        let st = self.get_client().prepare(query.as_str()).await?;

        self.query_one_trait(&st, &params).await
    }

    pub async fn remove_category(&self, cid: u32) -> Result<(), ResError> {
        let st = self
            .get_client()
            .prepare("DELETE FROM categories WHERE id=$1")
            .await?;

        self.get_client().execute(&st, &[&cid]).await?;
        Ok(())
    }

    pub async fn add_category(
        &self,
        c: CategoryRequest,
        g: &GlobalVars,
    ) -> Result<Category, ResError> {
        let st = self
            .get_client()
            .prepare(
                "
                INSERT INTO categories (id, name, thumbnail)
                VALUES ($1, $2, $3)
                RETURNING *",
            )
            .await?;

        let cid = g.lock().map(|mut lock| lock.next_cid()).await;

        self.query_one_trait(
            &st,
            &[
                &cid,
                c.name.as_ref().unwrap(),
                c.thumbnail.as_ref().unwrap(),
            ],
        )
        .await
    }
}

impl CacheService {
    pub fn get_categories_all(&self) -> impl Future<Output = Result<Vec<Category>, ResError>> {
        use crate::handler::cache::CategoriesFromCache;
        self.categories_from_cache_01().compat()
    }

    pub fn update_categories(&self, c: &[Category]) {
        actix::spawn(build_hmsets_01(self.get_conn(), c, CATEGORY_U8, false).map_err(|_| ()));
    }

    pub fn send_failed_category(&self, c: Category) {
        let _ = self
            .recipient
            .do_send(CacheFailedMessage::FailedCategory(c));
    }
}
