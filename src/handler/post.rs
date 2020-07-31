use std::future::Future;

use chrono::Utc;
use tokio_postgres::types::{ToSql, Type};

use crate::handler::{
    cache::{MyRedisPool, POST_U8},
    cache_update::{CacheFailedMessage, CacheServiceAddr},
    db::{GetStatement, MyPostgresPool, ParseRowStream},
};
use crate::model::{
    errors::ResError,
    post::{Post, PostRequest},
};

const INSERT_POST: &str =
    "INSERT INTO posts (id, user_id, topic_id, category_id, post_id, post_content, created_at, updated_at)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
    RETURNING *";

const INSERT_POST_TYPES: &[Type; 8] = &[
    Type::OID,
    Type::OID,
    Type::OID,
    Type::OID,
    Type::OID,
    Type::VARCHAR,
    Type::TIMESTAMP,
    Type::TIMESTAMP,
];

impl MyPostgresPool {
    pub async fn add_post(&self, p: PostRequest) -> Result<Vec<Post>, ResError> {
        let uid = p.user_id.as_ref().ok_or(ResError::BadRequest)?;
        let tid = p.topic_id.as_ref().ok_or(ResError::BadRequest)?;
        let content = p.post_content.as_ref().ok_or(ResError::BadRequest)?;

        let pool = self.get().await?;
        let (cli, _) = &*pool;

        let st = cli.prepare_typed(INSERT_POST, INSERT_POST_TYPES).await?;

        let id = crate::model::common::global().lock().next_pid();
        let now = &Utc::now().naive_local();
        let params: [&(dyn ToSql + Sync); 8] =
            [&id, uid, tid, &p.category_id, &p.post_id, content, now, now];

        cli.query_raw(&st, params.iter().map(|s| *s as _))
            .await?
            .parse_row()
            .await
    }

    pub async fn update_post(&self, p: PostRequest) -> Result<Vec<Post>, ResError> {
        let mut query = String::from("UPDATE posts SET");
        let mut params = Vec::new();
        let mut index = 1u8;

        if let Some(s) = p.topic_id.as_ref() {
            query.push_str(" topic_id=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &(dyn ToSql + Sync));
            index += 1;
        }
        if let Some(s) = p.post_id.as_ref() {
            query.push_str(" post_id=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &(dyn ToSql + Sync));
            index += 1;
        }
        if let Some(s) = p.post_content.as_ref() {
            query.push_str(" post_content=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &(dyn ToSql + Sync));
            index += 1;
        }
        if let Some(s) = p.is_locked.as_ref() {
            query.push_str(" is_locked=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &(dyn ToSql + Sync));
            index += 1;
        }

        if index == 1 {
            return Err(ResError::BadRequest);
        }

        query.push_str(" updated_at=DEFAULT WHERE id=$");
        query.push_str(index.to_string().as_str());
        params.push(p.id.as_ref().unwrap() as &(dyn ToSql + Sync));
        index += 1;

        if let Some(s) = p.user_id.as_ref() {
            query.push_str(" AND user_id=$");
            query.push_str(index.to_string().as_str());
            params.push(s as &(dyn ToSql + Sync));
        }
        query.push_str(" RETURNING *");

        let pool = self.get().await?;
        let (cli, _) = &*pool;

        let st = cli.prepare_typed(query.as_str(), &[]).await?;
        cli.query_raw(&st, params.iter().map(|s| *s as _))
            .await?
            .parse_row()
            .await
    }

    pub(crate) async fn get_posts(&self, pids: &[u32]) -> Result<(Vec<Post>, Vec<u32>), ResError> {
        let pool = self.get().await?;
        let (cli, sts) = &*pool;

        let st = sts.get_statement("posts_by_id")?;
        let params: [&(dyn ToSql + Sync); 1] = [&pids];

        let (p, uids) = cli
            .query_raw(st, params.iter().map(|s| *s as _))
            .await?
            .parse_row_with::<Post>()
            .await?;

        drop(pool);

        let p = Post::sort(p, &pids).await;

        Ok((p, uids))
    }
}

impl MyRedisPool {
    pub(crate) fn get_posts(
        &self,
        ids: Vec<u32>,
    ) -> impl Future<Output = Result<(Vec<Post>, Vec<u32>), ResError>> + '_ {
        self.get_cache_with_perm_with_uids(ids, crate::handler::cache::POST_U8)
    }

    pub(crate) async fn get_posts_old(
        &self,
        tid: u32,
        page: usize,
    ) -> Result<(Vec<Post>, Vec<u32>), ResError> {
        let key = format!("topic:{}:posts_time_created", tid);
        self.get_cache_with_uids_from_zrange(key.as_str(), page, crate::handler::cache::POST_U8)
            .await
    }

    pub(crate) async fn get_posts_pop(
        &self,
        tid: u32,
        page: usize,
    ) -> Result<(Vec<Post>, Vec<u32>), ResError> {
        let key = format!("topic:{}:posts_reply", tid);
        self.get_cache_with_uids_from_zrevrange_reverse_lex(
            key.as_str(),
            page,
            crate::handler::cache::POST_U8,
        )
        .await
    }

    pub(crate) async fn update_posts(&self, p: &[Post]) -> Result<(), ResError> {
        self.build_sets(p, POST_U8, true).await
    }

    pub(crate) async fn update_post_send_fail(&self, p: Vec<Post>, addr: CacheServiceAddr) {
        let r = self.build_sets(&p, POST_U8, true).await;
        if r.is_err() {
            if let Some(id) = p.first().map(|p| p.id) {
                let _ = addr.send(CacheFailedMessage::FailedPostUpdate(id)).await;
            }
        };
    }

    pub(crate) async fn add_post_send_fail(&self, p: Vec<Post>, addr: CacheServiceAddr) {
        if self.add_post(&p).await.is_err() {
            if let Some(id) = p.first().map(|p| p.id) {
                let _ = addr.send(CacheFailedMessage::FailedPost(id)).await;
            }
        }
    }
}
