use std::future::Future;

use chrono::Utc;
use futures::FutureExt;
use tokio_postgres::types::{ToSql, Type};

use crate::handler::{
    cache::MyRedisPool,
    cache::TOPIC_U8,
    cache_update::{CacheFailedMessage, SharedCacheUpdateAddr},
    db::MyPostgresPool,
};
use crate::model::{
    common::GlobalVars,
    errors::ResError,
    topic::{Topic, TopicRequest},
    user::User,
};

const INSERT_TOPIC: &str =
    "INSERT INTO topics (id, user_id, category_id, thumbnail, title, body, created_at, updated_at)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
    RETURNING *";

const INSERT_TOPIC_TYPES: &[Type; 8] = &[
    Type::OID,
    Type::OID,
    Type::OID,
    Type::VARCHAR,
    Type::VARCHAR,
    Type::VARCHAR,
    Type::TIMESTAMP,
    Type::TIMESTAMP,
];

impl MyPostgresPool {
    pub(crate) async fn add_topic(
        &self,
        t: &TopicRequest,
        g: &GlobalVars,
    ) -> Result<Topic, ResError> {
        let uid = t.user_id.as_ref().ok_or(ResError::BadRequest)?;
        let thumb = t.thumbnail.as_ref().ok_or(ResError::BadRequest)?;
        let title = t.title.as_ref().ok_or(ResError::BadRequest)?;
        let body = t.body.as_ref().ok_or(ResError::BadRequest)?;

        let mut pool = self.get_pool().await?;
        let mut cli = pool.get_client();

        let st = cli.prepare_typed(INSERT_TOPIC, INSERT_TOPIC_TYPES).await?;

        let id = g.lock().map(|mut lock| lock.next_tid()).await;
        let now = &Utc::now().naive_utc();

        cli.query_one(
            &st,
            &[&id, uid, &t.category_id, thumb, title, body, now, now],
        )
        .await
    }

    //ToDo: add query for moving topic to other table.
    pub(crate) async fn update_topic(&self, t: &TopicRequest) -> Result<Topic, ResError> {
        let mut query = String::from("UPDATE topics SET");
        let mut params = Vec::new();
        let mut index = 1u8;

        if let Some(s) = &t.title {
            query.push_str(" title=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &(dyn ToSql + Sync));
            index += 1;
        }
        if let Some(s) = &t.body {
            query.push_str(" body=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &(dyn ToSql + Sync));
            index += 1;
        }
        if let Some(s) = &t.thumbnail {
            query.push_str(" thumbnail=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &(dyn ToSql + Sync));
            index += 1;
        }
        if let Some(s) = &t.is_locked {
            query.push_str(" is_locked=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &(dyn ToSql + Sync));
            index += 1;
        }
        if let Some(s) = &t.is_visible {
            query.push_str(" is_visible=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &(dyn ToSql + Sync));
            index += 1;
        }
        // update update_at or return err as the query is empty.
        if index == 1 {
            return Err(ResError::BadRequest);
        }

        query.push_str(" updated_at=DEFAULT WHERE id=$");
        query.push_str(index.to_string().as_str());
        params.push(t.id.as_ref().unwrap() as &(dyn ToSql + Sync));
        index += 1;

        if let Some(s) = t.user_id.as_ref() {
            query.push_str(" AND user_id=$");
            query.push_str(index.to_string().as_str());
            params.push(s as &(dyn ToSql + Sync));
        }
        query.push_str(" RETURNING *");

        let mut pool = self.get_pool().await?;
        let mut cli = pool.get_client();

        let st = cli.prepare(query.as_str()).await?;
        cli.query_one(&st, params.as_slice()).await
    }

    pub(crate) async fn get_topics_with_users(
        &self,
        ids: &[u32],
    ) -> Result<(Vec<Topic>, Vec<User>), ResError> {
        let mut pool = self.get_pool().await?;
        let (mut cli, sts) = pool.get_client_statements();

        let st = sts.get_statement(0)?;
        let (t, mut uids) = cli.query_multi_with_uid(st, ids).await?;

        uids.sort();
        uids.dedup();

        let st = sts.get_statement(2)?;
        let u = cli
            .query_multi(st, &[&uids], Vec::with_capacity(21))
            .await?;

        drop(pool);
        let t = Topic::sort(t, &ids).await;

        Ok((t, u))
    }

    pub(crate) async fn get_topics(
        &self,
        tids: &[u32],
    ) -> Result<(Vec<Topic>, Vec<u32>), ResError> {
        let mut pool = self.get_pool().await?;
        let (mut cli, sts) = pool.get_client_statements();

        let st = sts.get_statement(0)?;
        let (t, uids) = cli.query_multi_with_uid(st, tids).await?;

        drop(pool);

        let t = Topic::sort(t, &tids).await;

        Ok((t, uids))
    }
}

impl MyRedisPool {
    pub(crate) async fn get_topics_pop(
        &self,
        cid: u32,
        page: usize,
    ) -> Result<(Vec<Topic>, Vec<u32>), ResError> {
        let key = format!("category:{}:list_pop", cid);
        self.get_cache_with_uids_from_list(key.as_str(), page, crate::handler::cache::TOPIC_U8)
            .await
    }

    pub(crate) fn get_topics_pop_all(
        &self,
        page: usize,
    ) -> impl Future<Output = Result<(Vec<Topic>, Vec<u32>), ResError>> + '_ {
        self.get_cache_with_uids_from_list(
            "category:all:list_pop",
            page,
            crate::handler::cache::TOPIC_U8,
        )
    }

    pub(crate) async fn get_topics_late(
        &self,
        cid: u32,
        page: usize,
    ) -> Result<(Vec<Topic>, Vec<u32>), ResError> {
        let key = format!("category:{}:topics_time", cid);
        self.get_cache_with_uids_from_zrevrange(key.as_str(), page, crate::handler::cache::TOPIC_U8)
            .await
    }

    pub(crate) fn get_topics(
        &self,
        ids: Vec<u32>,
    ) -> impl Future<Output = Result<(Vec<Topic>, Vec<u32>), ResError>> + '_ {
        self.get_cache_with_perm_with_uids(ids, crate::handler::cache::TOPIC_U8)
    }

    pub(crate) async fn update_topics(&self, t: &[Topic]) -> Result<(), ResError> {
        self.build_sets(t, TOPIC_U8, true).await
    }

    pub(crate) async fn update_topic_send_fail(
        &self,
        t: Topic,
        addr: SharedCacheUpdateAddr,
    ) -> Result<(), ()> {
        let id = t.id;
        let r = self.build_sets(&[t], TOPIC_U8, true).await;
        if r.is_err() {
            addr.do_send(CacheFailedMessage::FailedTopicUpdate(id))
                .await;
        };
        Ok(())
    }

    pub(crate) async fn add_topic_send_fail(
        &self,
        t: Topic,
        addr: SharedCacheUpdateAddr,
    ) -> Result<(), ()> {
        if self.add_topic(&t).await.is_err() {
            addr.do_send(CacheFailedMessage::FailedTopic(t.id)).await;
        }
        Ok(())
    }
}
