use std::fmt::Write;
use std::future::Future;

use chrono::Utc;
use futures::FutureExt;
use futures01::Future as Future01;

use crate::handler::{
    cache::{build_hmsets_01, CacheService, GetSharedConn, POST_U8},
    cache_update::CacheFailedMessage,
    db::DatabaseService,
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

        use crate::handler::db::Query;
        self.query_one_trait(
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

        if let Some(s) = p.topic_id {
            let _ = write!(&mut query, " topic_id = {},", s);
        }
        if let Some(s) = p.post_id {
            let _ = write!(&mut query, " post_id = {},", s);
        }
        if let Some(s) = p.post_content {
            let _ = write!(&mut query, " post_content = '{}',", s);
        }
        if let Some(s) = p.is_locked {
            let _ = write!(&mut query, " is_locked = {},", s);
        }

        if query.ends_with(',') {
            let _ = write!(
                &mut query,
                " updated_at = DEFAULT WHERE id = {}",
                p.id.unwrap()
            );
        } else {
            return Err(ResError::BadRequest);
        }

        if let Some(s) = p.user_id {
            let _ = write!(&mut query, " AND user_id = {}", s);
        }
        query.push_str(" RETURNING *");

        use crate::handler::db::SimpleQuery;
        self.simple_query_one_trait(query.as_str()).await
    }

    pub async fn get_posts_with_uid(&self, ids: &[u32]) -> Result<(Vec<Post>, Vec<u32>), ResError> {
        let st = self.posts_by_id.borrow();
        self.get_by_id_with_uid(&st, ids).await
    }
}

impl CacheService {
    pub fn get_posts_from_ids(
        &self,
        ids: Vec<u32>,
    ) -> impl Future<Output = Result<(Vec<Post>, Vec<u32>), ResError>> {
        self.get_cache_with_uids_from_ids(ids, crate::handler::cache::POST_U8)
    }

    pub fn get_posts_old(
        &self,
        tid: u32,
        page: usize,
    ) -> impl Future<Output = Result<(Vec<Post>, Vec<u32>), ResError>> {
        self.get_cache_with_uids_from_zrange(
            &format!("topic:{}:posts_time_created", tid),
            page,
            crate::handler::cache::POST_U8,
        )
    }

    pub fn get_posts_pop(
        &self,
        tid: u32,
        page: usize,
    ) -> impl Future<Output = Result<(Vec<Post>, Vec<u32>), ResError>> {
        self.get_cache_with_uids_from_zrevrange_reverse_lex(
            &format!("topic:{}:posts_reply", tid),
            page,
            crate::handler::cache::POST_U8,
        )
    }

    pub fn update_posts(&self, t: &[Post]) {
        actix::spawn(build_hmsets_01(self.get_conn(), t, POST_U8, true).map_err(|_| ()));
    }

    pub fn update_post_return_fail(
        &self,
        p: Vec<Post>,
    ) -> impl Future01<Item = (), Error = Vec<Post>> {
        build_hmsets_01(self.get_conn(), &p, POST_U8, true).map_err(|_| p)
    }

    pub fn send_failed_post(&self, p: Post) {
        let _ = self.recipient.do_send(CacheFailedMessage::FailedPost(p));
    }

    pub fn send_failed_post_update(&self, p: Vec<Post>) {
        let _ = self
            .recipient
            .do_send(CacheFailedMessage::FailedPostUpdate(p));
    }
}
