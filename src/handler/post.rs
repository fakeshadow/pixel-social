use std::future::Future;

use chrono::Utc;
use futures::FutureExt;
use tokio_postgres::types::{ToSql, Type};

use crate::handler::{
    cache::{MyRedisPool, POST_U8},
    cache_update::{CacheFailedMessage, SharedCacheUpdateAddr},
    db::MyPostgresPool,
};
use crate::model::{
    common::GlobalVars,
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
    pub async fn add_post(&self, p: PostRequest, g: &GlobalVars) -> Result<Post, ResError> {
        let uid = p.user_id.as_ref().ok_or(ResError::BadRequest)?;
        let tid = p.topic_id.as_ref().ok_or(ResError::BadRequest)?;
        let content = p.post_content.as_ref().ok_or(ResError::BadRequest)?;

        let mut pool_ref = self.get_pool().await?;
        let mut cli = pool_ref.get_client();

        let st = cli.prepare_typed(INSERT_POST, INSERT_POST_TYPES).await?;

        let id = g.lock().map(|mut lock| lock.next_pid()).await;

        let now = &Utc::now().naive_local();

        cli.query_one(
            &st,
            &[&id, uid, tid, &p.category_id, &p.post_id, content, now, now],
        )
        .await
    }

    pub async fn update_post(&self, p: PostRequest) -> Result<Post, ResError> {
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

        let mut pool_ref = self.get_pool().await?;
        let mut cli = pool_ref.get_client();

        let st = cli.prepare(query.as_str()).await?;
        cli.query_one(&st, params.as_slice()).await
    }

    pub(crate) async fn get_posts(&self, pids: &[u32]) -> Result<(Vec<Post>, Vec<u32>), ResError> {
        let mut pool_ref = self.get_pool().await?;
        let (mut cli, sts) = pool_ref.get_client_statements();

        let st = sts.get_statement(1)?;
        let (p, uids) = cli.query_multi_with_uid(st, pids).await?;

        drop(pool_ref);

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

    pub(crate) async fn update_post_send_fail(
        &self,
        p: Post,
        addr: SharedCacheUpdateAddr,
    ) -> Result<(), ()> {
        let id = p.id;
        let r = self.build_sets(&[p], POST_U8, true).await;
        if r.is_err() {
            addr.do_send(CacheFailedMessage::FailedPostUpdate(id)).await;
        };
        Ok(())
    }

    pub(crate) async fn add_post_send_fail(
        &self,
        p: Post,
        addr: SharedCacheUpdateAddr,
    ) -> Result<(), ()> {
        if self.add_post(&p).await.is_err() {
            addr.do_send(CacheFailedMessage::FailedPost(p.id)).await;
        }
        Ok(())
    }
}
