use std::future::Future;

use chrono::Utc;
use futures::{FutureExt, TryFutureExt};
use tokio_postgres::types::ToSql;

use crate::handler::{
    cache::{build_hmsets, CacheService, GetSharedConn, POST_U8},
    cache_update::CacheFailedMessage,
    db::{AsCrateClient, DatabaseService},
};
use crate::model::{
    common::GlobalVars,
    errors::ResError,
    post::{Post, PostRequest},
};

impl DatabaseService {
    pub async fn add_post(&self, p: PostRequest, g: &GlobalVars) -> Result<Post, ResError> {
        let id = g.lock().map(|mut lock| lock.next_pid()).await;

        let now = &Utc::now().naive_local();

        self.client
            .borrow_mut()
            .as_cli()
            .query_one(
                &self.insert_post.borrow(),
                &[
                    &id,
                    p.user_id.as_ref().unwrap(),
                    &p.topic_id.as_ref().unwrap(),
                    &p.category_id,
                    &p.post_id,
                    p.post_content.as_ref().unwrap(),
                    now,
                    now,
                ],
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
            params.push(s as &dyn ToSql);
            index += 1;
        }
        if let Some(s) = p.post_id.as_ref() {
            query.push_str(" post_id=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &dyn ToSql);
            index += 1;
        }
        if let Some(s) = p.post_content.as_ref() {
            query.push_str(" post_content=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &dyn ToSql);
            index += 1;
        }
        if let Some(s) = p.is_locked.as_ref() {
            query.push_str(" is_locked=$");
            query.push_str(index.to_string().as_str());
            query.push_str(",");
            params.push(s as &dyn ToSql);
            index += 1;
        }

        if index == 1 {
            return Err(ResError::BadRequest);
        }

        query.push_str(" updated_at=DEFAULT WHERE id=$");
        query.push_str(index.to_string().as_str());
        params.push(p.id.as_ref().unwrap() as &dyn ToSql);
        index += 1;

        if let Some(s) = p.user_id.as_ref() {
            query.push_str(" AND user_id=$");
            query.push_str(index.to_string().as_str());
            params.push(s as &dyn ToSql);
        }
        query.push_str(" RETURNING *");

        let mut r = self.client.borrow_mut();
        let mut c = r.as_cli();

        let st = c.prep(query.as_str()).await?;
        c.query_one(&st, &params).await
    }

    pub async fn get_posts_with_uid(&self, ids: &[u32]) -> Result<(Vec<Post>, Vec<u32>), ResError> {
        let st = &*self.posts_by_id.borrow();
        self.get_by_id_with_uid(st, ids).await
    }
}

impl CacheService {
    pub fn get_posts_from_ids(
        &self,
        ids: Vec<u32>,
    ) -> impl Future<Output = Result<(Vec<Post>, Vec<u32>), ResError>> + '_ {
        self.get_cache_with_uids_from_ids(ids, crate::handler::cache::POST_U8)
    }

    pub async fn get_posts_old(
        &self,
        tid: u32,
        page: usize,
    ) -> Result<(Vec<Post>, Vec<u32>), ResError> {
        let key = format!("topic:{}:posts_time_created", tid);
        self.get_cache_with_uids_from_zrange(key.as_str(), page, crate::handler::cache::POST_U8)
            .await
    }

    pub async fn get_posts_pop(
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

    pub fn update_posts(&self, t: &[Post]) {
        let conn = self.get_conn();
        tokio::spawn(build_hmsets(conn, t, POST_U8, true).map(|_| ()));
    }

    pub fn update_post_return_fail(
        &self,
        p: Vec<Post>,
    ) -> impl Future<Output = Result<(), Vec<Post>>> {
        let conn = self.get_conn();
        build_hmsets(conn, &p, POST_U8, true).map_err(|_| p)
    }

    pub fn send_failed_post(&self, p: Post) {
        let _ = self.addr.do_send(CacheFailedMessage::FailedPost(p));
    }

    pub fn send_failed_post_update(&self, p: Vec<Post>) {
        let _ = self.addr.do_send(CacheFailedMessage::FailedPostUpdate(p));
    }
}
