use futures::{FutureExt, TryFutureExt};
use tokio_postgres::types::ToSql;

use crate::handler::cache::{FromCache, IdsFromList};
use crate::handler::{
    cache::{build_hmsets, CacheService, GetSharedConn, CATEGORY_U8},
    cache_update::CacheFailedMessage,
    db::{AsCrateClient, DatabaseService},
};
use crate::model::{
    category::{Category, CategoryRequest},
    common::GlobalVars,
    errors::ResError,
};

impl DatabaseService {
    pub async fn get_categories_all(&self) -> Result<Vec<Category>, ResError> {
        let st = self
            .client
            .borrow_mut()
            .as_cli()
            .prep("SELECT * FROM categories")
            .await?;

        self.client
            .borrow_mut()
            .as_cli()
            .query_multi(&st, &[], Vec::new())
            .await
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

        let st = self
            .client
            .borrow_mut()
            .as_cli()
            .prep(query.as_str())
            .await?;

        self.client
            .borrow_mut()
            .as_cli()
            .query_one(&st, &params)
            .await
    }

    pub async fn remove_category(&self, cid: u32) -> Result<(), ResError> {
        let st = self
            .client
            .borrow_mut()
            .as_cli()
            .prep("DELETE FROM categories WHERE id=$1")
            .await?;

        self.client.borrow_mut().as_cli().exec(&st, &[&cid]).await?;

        Ok(())
    }

    pub async fn add_category(
        &self,
        c: CategoryRequest,
        g: &GlobalVars,
    ) -> Result<Category, ResError> {
        let st = self
            .client
            .borrow_mut()
            .as_cli()
            .prep(
                "
                INSERT INTO categories (id, name, thumbnail)
                VALUES ($1, $2, $3)
                RETURNING *",
            )
            .await?;

        let cid = g.lock().map(|mut lock| lock.next_cid()).await;

        self.client
            .borrow_mut()
            .as_cli()
            .query_one(
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
    pub async fn get_categories_all(&self) -> Result<Vec<Category>, ResError> {
        let (conn, vec): (_, Vec<u32>) =
            self.ids_from_cache_list("category_id:meta", 0, 999).await?;
        CacheService::from_cache(conn, vec, CATEGORY_U8, false).await
    }

    pub fn update_categories(&self, c: &[Category]) {
        actix::spawn(
            build_hmsets(self.get_conn(), c, CATEGORY_U8, false)
                .map_err(|_| ())
                .boxed_local()
                .compat(),
        );
    }

    pub fn send_failed_category(&self, c: Category) {
        let _ = self.addr.do_send(CacheFailedMessage::FailedCategory(c));
    }
}
