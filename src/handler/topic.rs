use std::fmt::Write;

use futures::{
    Future,
    future::{
        err as ft_err,
        Either,
    },
};

use chrono::Utc;

use crate::model::{
    topic::TopicRequest,
    common::GlobalVars,
    errors::ResError,
    topic::Topic,
};
use crate::handler::db::DatabaseServiceRaw;
use crate::handler::cache::CacheServiceRaw;

impl DatabaseServiceRaw {
    pub fn add_topic(
        &self,
        t: TopicRequest,
        g: GlobalVars,
    ) -> impl Future<Item=Topic, Error=ResError> {
        let id = match g.lock() {
            Ok(mut var) => var.next_tid(),
            Err(_) => return Either::A(ft_err(ResError::InternalServerError))
        };
        let now = &Utc::now().naive_utc();

        use crate::handler::db::QueryRaw;
        Either::B(self
            .query_one_trait(
                &self.insert_topic,
                &[
                    &id,
                    &t.user_id.unwrap(),
                    &t.category_id,
                    &t.thumbnail.unwrap(),
                    &t.title.unwrap(),
                    &t.body.unwrap(),
                    now,
                    now
                ],
            )
        )
    }
    //ToDo: add query for moving topic to other table.
    pub fn update_topic(
        &self,
        t: TopicRequest,
    ) -> impl Future<Item=Topic, Error=ResError> {
        let mut query = String::from("UPDATE topics SET");

        if let Some(s) = t.title {
            let _ = write!(&mut query, " title='{}',", s);
        }
        if let Some(s) = t.body {
            let _ = write!(&mut query, " body='{}',", s);
        }
        if let Some(s) = t.thumbnail {
            let _ = write!(&mut query, " thumbnail='{}',", s);
        }
        if let Some(s) = t.is_locked {
            let _ = write!(&mut query, " is_locked={},", s);
        }
        // update update_at or return err as the query is empty.
        if query.ends_with(",") {
            let _ = write!(&mut query, " updated_at=DEFAULT");
        } else {
            return Either::A(ft_err(ResError::BadRequest));
        }

        let _ = write!(&mut query, " WHERE id={} ", t.id.unwrap());
        if let Some(s) = t.user_id {
            let _ = write!(&mut query, "AND user_id={} ", s);
        }
        query.push_str("RETURNING *");

        use crate::handler::db::SimpleQueryRaw;
        Either::B(self.simple_query_one_trait(query.as_str()))
    }
}


impl CacheServiceRaw {
    pub fn get_topics_pop(
        &self,
        cid: u32,
        page: i64,
    ) -> impl Future<Item=(Vec<Topic>, Vec<u32>), Error=ResError> {
        self.get_cache_with_uids_from_list(&format!("category:{}:list_pop", cid), page, "topic")
    }

    pub fn get_topics_pop_all(
        &self,
        page: i64,
    ) -> impl Future<Item=(Vec<Topic>, Vec<u32>), Error=ResError> {
        self.get_cache_with_uids_from_list("category:all:list_pop", page, "topic")
    }

    pub fn get_topics_late(
        &self,
        cid: u32,
        page: i64,
    ) -> impl Future<Item=(Vec<Topic>, Vec<u32>), Error=ResError> {
        self.get_cache_with_uids_from_zrevrange(&format!("category:{}:topics_time", cid), page, "topic")
    }

    pub fn get_topics_from_ids(
        &self,
        ids: Vec<u32>,
    ) -> impl Future<Item=(Vec<Topic>, Vec<u32>), Error=ResError> {
        self.get_cache_with_uids_from_ids(ids, "topic")
    }
}