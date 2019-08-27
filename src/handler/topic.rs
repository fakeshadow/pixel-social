use std::fmt::Write;
use std::future::Future;

use chrono::Utc;
use futures01::Future as Future01;
use futures::FutureExt;

use crate::handler::{
    cache::{build_hmsets_01, CacheService, GetSharedConn, TOPIC_U8},
    cache_update::CacheFailedMessage,
    db::DatabaseService,
};
use crate::model::{
    common::GlobalVars,
    errors::ResError,
    topic::{Topic, TopicRequest},
};

impl DatabaseService {
    pub async fn add_topic(
        &self,
        t: &TopicRequest,
        g: &GlobalVars,
    ) -> Result<Topic, ResError> {
        let id = g.lock().map(|mut lock| lock.next_tid()).await;

        let now = &Utc::now().naive_utc();

        use crate::handler::db::Query;
        self.query_one_trait(
            &self.insert_topic.borrow(),
            &[
                &id,
                t.user_id.as_ref().unwrap(),
                &t.category_id,
                t.thumbnail.as_ref().unwrap(),
                t.title.as_ref().unwrap(),
                t.body.as_ref().unwrap(),
                now,
                now
            ],
        ).await
    }

    //ToDo: add query for moving topic to other table.
    pub async fn update_topic(&self, t: &TopicRequest) -> Result<Topic, ResError> {
        let mut query = String::from("UPDATE topics SET");

        if let Some(s) = &t.title {
            let _ = write!(&mut query, " title='{}',", s);
        }
        if let Some(s) = &t.body {
            let _ = write!(&mut query, " body='{}',", s);
        }
        if let Some(s) = &t.thumbnail {
            let _ = write!(&mut query, " thumbnail='{}',", s);
        }
        if let Some(s) = &t.is_locked {
            let _ = write!(&mut query, " is_locked={},", s);
        }
        if let Some(s) = &t.is_visible {
            let _ = write!(&mut query, " is_visible={},", s);
        }
        // update update_at or return err as the query is empty.
        if query.ends_with(',') {
            let _ = write!(&mut query, " updated_at=DEFAULT");
        } else {
            return Err(ResError::BadRequest);
        }

        let _ = write!(&mut query, " WHERE id={} ", t.id.unwrap());
        if let Some(s) = t.user_id {
            let _ = write!(&mut query, "AND user_id={} ", s);
        }
        query.push_str("RETURNING *");

        use crate::handler::db::SimpleQuery;
        self.simple_query_one_trait(query.as_str()).await
    }

    pub async fn get_topics_with_uid(&self, ids: &[u32]) -> Result<(Vec<Topic>, Vec<u32>), ResError> {
        let st = &self.topics_by_id.borrow();
        self.get_by_id_with_uid(st, ids).await
    }
}

impl CacheService {
    pub fn get_topics_pop(
        &self,
        cid: u32,
        page: usize,
    ) -> impl Future<Output=Result<(Vec<Topic>, Vec<u32>), ResError>> {
        self.get_cache_with_uids_from_list(
            &format!("category:{}:list_pop", cid),
            page,
            crate::handler::cache::TOPIC_U8,
        )
    }

    pub fn get_topics_pop_all(
        &self,
        page: usize,
    ) -> impl Future<Output=Result<(Vec<Topic>, Vec<u32>), ResError>> {
        self.get_cache_with_uids_from_list(
            "category:all:list_pop",
            page,
            crate::handler::cache::TOPIC_U8,
        )
    }

    pub fn get_topics_late(
        &self,
        cid: u32,
        page: usize,
    ) -> impl Future<Output=Result<(Vec<Topic>, Vec<u32>), ResError>> {
        self.get_cache_with_uids_from_zrevrange(
            &format!("category:{}:topics_time", cid),
            page,
            crate::handler::cache::TOPIC_U8,
        )
    }

    pub fn get_topics_from_ids(
        &self,
        ids: Vec<u32>,
    ) -> impl Future<Output=Result<(Vec<Topic>, Vec<u32>), ResError>> {
        self.get_cache_with_uids_from_ids(ids, crate::handler::cache::TOPIC_U8)
    }

    pub fn update_topics(&self, t: &[Topic]) {
        actix::spawn(build_hmsets_01(self.get_conn(), t, TOPIC_U8, true).map_err(|_| ()));
    }

    // Don't confused these with update_topics/posts/users methods. The latter methods run in spawned futures and the errors are ignored.
    // They are separate methods as we don't want to retry every failed update cache for most times the data are from expired cache query and not actual content update.
    pub fn update_topic_return_fail(&self, t: Vec<Topic>) -> impl Future01<Item=(), Error=Vec<Topic>> {
        build_hmsets_01(self.get_conn(), &t, TOPIC_U8, true).map_err(|_| t)
    }

    // send failed data to CacheUpdateService actor and retry from there.
    pub fn send_failed_topic(&self, t: Topic) { let _ = self.recipient.do_send(CacheFailedMessage::FailedTopic(t)); }

    pub fn send_failed_topic_update(&self, t: Vec<Topic>) { let _ = self.recipient.do_send(CacheFailedMessage::FailedTopicUpdate(t)); }
}