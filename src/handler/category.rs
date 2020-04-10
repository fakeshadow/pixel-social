use std::future::Future;

use futures::FutureExt;
use tokio_postgres::types::ToSql;

use crate::handler::cache_update::CacheServiceAddr;
use crate::handler::{
    cache::{MyRedisPool, CATEGORY_U8},
    cache_update::CacheFailedMessage,
    db::{MyPostgresPool, ParseRowStream},
};
use crate::model::{
    category::{Category, CategoryRequest},
    errors::ResError,
};

const GET_CATEGORY_ALL: &str = "SELECT * FROM categories";
const GET_CATEGORY: &str = "SELECT * FROM categories WHERE id=ANY($1)";
const DEL_CATEGORY: &str = "DELETE FROM categories WHERE id=$1";
const INSERT_CATEGORY: &str =
    "INSERT INTO categories (id, name, thumbnail) VALUES ($1, $2, $3) RETURNING *";

impl MyPostgresPool {
    pub(crate) async fn get_categories_all(&self) -> Result<Vec<Category>, ResError> {
        let pool = self.get().await?;
        let (cli, _) = &*pool;

        let st = cli.prepare_typed(GET_CATEGORY_ALL, &[]).await?;
        let params: [&(dyn ToSql + Sync); 0] = [];

        cli.query_raw(&st, params.iter().map(|s| *s as _))
            .await?
            .parse_row()
            .await
    }

    pub(crate) async fn get_categories(&self, ids: &[u32]) -> Result<Vec<Category>, ResError> {
        let pool = self.get().await?;
        let (cli, _) = &*pool;

        let st = cli.prepare_typed(GET_CATEGORY, &[]).await?;
        let params: [&(dyn ToSql + Sync); 1] = [&ids];

        cli.query_raw(&st, params.iter().map(|s| *s as _))
            .await?
            .parse_row()
            .await
    }

    pub(crate) async fn add_category(&self, c: CategoryRequest) -> Result<Vec<Category>, ResError> {
        let name = c.name.as_ref().ok_or(ResError::BadRequest)?;
        let thumb = c.thumbnail.as_ref().ok_or(ResError::BadRequest)?;

        let pool = self.get().await?;
        let (cli, _) = &*pool;

        let st = cli.prepare_typed(INSERT_CATEGORY, &[]).await?;

        let cid = crate::model::common::GLOBALS
            .lock()
            .map(|mut lock| lock.next_cid())
            .await;
        let params: [&(dyn ToSql + Sync); 3] = [&cid, &name, &thumb];

        cli.query_raw(&st, params.iter().map(|s| *s as _))
            .await?
            .parse_row()
            .await
    }

    pub(crate) async fn update_category(
        &self,
        c: CategoryRequest,
    ) -> Result<Vec<Category>, ResError> {
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

        let pool = self.get().await?;
        let (cli, _) = &*pool;

        let st = cli.prepare_typed(query.as_str(), &[]).await?;

        cli.query_raw(&st, params.iter().map(|s| *s as _))
            .await?
            .parse_row()
            .await
    }

    pub async fn remove_category(&self, cid: u32) -> Result<(), ResError> {
        let pool = self.get().await?;
        let (cli, _) = &*pool;

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

    pub(crate) async fn add_category_send_fail(&self, c: Vec<Category>, addr: CacheServiceAddr) {
        if self.add_category(&c).await.is_err() {
            if let Some(id) = c.first().map(|c| c.id) {
                let _ = addr.send(CacheFailedMessage::FailedCategory(id)).await;
            }
        };
    }
}
