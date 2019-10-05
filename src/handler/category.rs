use std::future::Future;

use futures::FutureExt;
use tokio_postgres::types::ToSql;

use crate::handler::cache_update::SharedCacheUpdateAddr;
use crate::handler::{
    cache::{MyRedisPool, CATEGORY_U8},
    cache_update::CacheFailedMessage,
    db::MyPostgresPool,
};
use crate::model::{
    category::{Category, CategoryRequest},
    common::GlobalVars,
    errors::ResError,
};

const DEL_CATEGORY: &str = "DELETE FROM categories WHERE id=$1";
const INSERT_CATEGORY: &str =
    "INSERT INTO categories (id, name, thumbnail) VALUES ($1, $2, $3) RETURNING *";

impl MyPostgresPool {
    pub(crate) async fn get_categories_all(&self) -> Result<Vec<Category>, ResError> {
        let mut pool_ref = self.get_pool().await?;

        let mut cli = pool_ref.get_client();

        let st = cli.prepare("SELECT * FROM categories").await?;
        cli.query_multi(&st, &[], Vec::new()).await
    }

    pub(crate) async fn get_categories(&self, ids: &[u32]) -> Result<Vec<Category>, ResError> {
        let mut pool_ref = self.get_pool().await?;

        let mut cli = pool_ref.get_client();

        let st = cli
            .prepare("SELECT * FROM categories WHERE id=ANY($1)")
            .await?;
        cli.query_multi(&st, &[&ids], Vec::new()).await
    }

    pub(crate) async fn add_category(
        &self,
        c: CategoryRequest,
        g: &GlobalVars,
    ) -> Result<Category, ResError> {
        let name = c.name.as_ref().ok_or(ResError::BadRequest)?;
        let thumb = c.thumbnail.as_ref().ok_or(ResError::BadRequest)?;

        let mut pool_ref = self.get_pool().await?;
        let mut cli = pool_ref.get_client();

        let st = cli.prepare(INSERT_CATEGORY).await?;
        let cid = g.lock().map(|mut lock| lock.next_cid()).await;

        cli.query_one(&st, &[&cid, &name, &thumb]).await
    }

    pub(crate) async fn update_category(&self, c: CategoryRequest) -> Result<Category, ResError> {
        let mut query = String::from("UPDATE categories SET");
        let mut params = Vec::new();
        let mut index = 1u8;

        if let Some(s) = c.thumbnail.as_ref() {
            query.push_str(" thumbnail=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &(dyn ToSql + Sync));
            index += 1;
        }
        if let Some(s) = c.name.as_ref() {
            query.push_str(" name=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &(dyn ToSql + Sync));
            index += 1;
        }

        if query.ends_with(',') {
            query.pop();
            query.push_str(" WHERE id=$");
            query.push_str(index.to_string().as_str());
            params.push(c.id.as_ref().unwrap() as &(dyn ToSql + Sync));
        } else {
            return Err(ResError::BadRequest);
        };

        query.push_str(" RETURNING *");

        let mut pool_ref = self.get_pool().await?;
        let mut cli = pool_ref.get_client();

        let st = cli.prepare(query.as_str()).await?;
        cli.query_one(&st, params.as_slice()).await
    }

    pub async fn remove_category(&self, cid: u32) -> Result<(), ResError> {
        let mut pool_ref = self.get_pool().await?;
        let mut cli = pool_ref.get_client();

        let st = cli.prepare(DEL_CATEGORY).await?;
        cli.execute(&st, &[&cid]).await?;

        Ok(())
    }
}

impl MyRedisPool {
    pub(crate) fn get_categories_all(
        &self,
    ) -> impl Future<Output = Result<Vec<Category>, ResError>> + '_ {
        self.get_cache_from_list("category_id:meta", CATEGORY_U8, 0, 999, false)
    }

    pub(crate) async fn update_categories(&self, c: &[Category]) -> Result<(), ResError> {
        self.build_sets(c, CATEGORY_U8, false).await
    }

    pub(crate) async fn add_category_send_fail(
        &self,
        c: Category,
        addr: SharedCacheUpdateAddr,
    ) -> Result<(), ()> {
        if self.add_category(&c).await.is_err() {
            addr.do_send(CacheFailedMessage::FailedCategory(c.id)).await;
        }
        Ok(())
    }
}
