use std::fmt::Write;

use chrono::Utc;
use futures::{
    future::{err as ft_err, Either},
    Future,
};

use crate::handler::{cache::CacheService, db::DatabaseService};
use crate::model::{common::GlobalVars, errors::ResError, topic::Topic, topic::TopicRequest};

impl DatabaseService {
    pub fn add_topic(
        &self,
        t: &TopicRequest,
        g: &GlobalVars,
    ) -> impl Future<Item = Topic, Error = ResError> {
        let id = match g.lock() {
            Ok(mut var) => var.next_tid(),
            Err(_) => return Either::A(ft_err(ResError::InternalServerError)),
        };
        let now = &Utc::now().naive_utc();

        use crate::handler::db::Query;
        Either::B(self.query_one_trait(
            &self.insert_topic.borrow(),
            &[
                &id,
                t.user_id.as_ref().unwrap(),
                &t.category_id,
                t.thumbnail.as_ref().unwrap(),
                t.title.as_ref().unwrap(),
                t.body.as_ref().unwrap(),
                now,
                now,
            ],
        ))
    }
    //ToDo: add query for moving topic to other table.
    pub fn update_topic(&self, t: &TopicRequest) -> impl Future<Item = Topic, Error = ResError> {
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
            return Either::A(ft_err(ResError::BadRequest));
        }

        let _ = write!(&mut query, " WHERE id={} ", t.id.unwrap());
        if let Some(s) = t.user_id {
            let _ = write!(&mut query, "AND user_id={} ", s);
        }
        query.push_str("RETURNING *");

        use crate::handler::db::SimpleQuery;
        Either::B(self.simple_query_one_trait(query.as_str()))
    }

    pub fn get_topics_by_id_with_uid(
        &self,
        ids: Vec<u32>,
    ) -> impl Future<Item = (Vec<Topic>, Vec<u32>), Error = ResError> {
        self.get_by_id_with_uid(&self.topics_by_id.borrow(), ids)
    }
}

impl CacheService {
    pub fn get_topics_pop(
        &self,
        cid: u32,
        page: usize,
    ) -> impl Future<Item = (Vec<Topic>, Vec<u32>), Error = ResError> {
        self.get_cache_with_uids_from_list(
            &format!("category:{}:list_pop", cid),
            page,
            crate::handler::cache::TOPIC_U8,
        )
    }

    pub fn get_topics_pop_all(
        &self,
        page: usize,
    ) -> impl Future<Item = (Vec<Topic>, Vec<u32>), Error = ResError> {
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
    ) -> impl Future<Item = (Vec<Topic>, Vec<u32>), Error = ResError> {
        self.get_cache_with_uids_from_zrevrange(
            &format!("category:{}:topics_time", cid),
            page,
            crate::handler::cache::TOPIC_U8,
        )
    }

    pub fn get_topics_from_ids(
        &self,
        ids: Vec<u32>,
    ) -> impl Future<Item = (Vec<Topic>, Vec<u32>), Error = ResError> {
        self.get_cache_with_uids_from_ids(ids, crate::handler::cache::TOPIC_U8)
    }
}
