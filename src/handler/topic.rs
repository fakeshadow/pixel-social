use std::fmt::Write;
use futures::future::err as ft_err;

use actix::prelude::{
    AsyncContext,
    Handler,
    Message,
    ResponseFuture,
    WrapFuture,
};
use chrono::Utc;

use crate::model::{
    actors::{DatabaseService, CacheService},
    topic::TopicRequest,
    common::GlobalVars,
    errors::ResError,
    topic::Topic,
};

pub struct AddTopic(pub TopicRequest, pub GlobalVars);

pub struct UpdateTopic(pub TopicRequest);

pub struct GetTopics(pub Vec<u32>);

impl Message for AddTopic {
    type Result = Result<Topic, ResError>;
}

impl Message for UpdateTopic {
    type Result = Result<Topic, ResError>;
}

impl Message for GetTopics {
    type Result = Result<(Vec<Topic>, Vec<u32>), ResError>;
}

impl Handler<AddTopic> for DatabaseService {
    type Result = ResponseFuture<Topic, ResError>;

    fn handle(&mut self, msg: AddTopic, _: &mut Self::Context) -> Self::Result {
        let id = match msg.1.lock() {
            Ok(mut var) => var.next_tid(),
            Err(_) => return Box::new(ft_err(ResError::InternalServerError))
        };
        let t = msg.0;
        let now = &Utc::now().naive_utc();

        Box::new(self
            .insert_topic(&[
                &id,
                &t.user_id.unwrap(),
                &t.category_id,
                &t.thumbnail.unwrap(),
                &t.title.unwrap(),
                &t.body.unwrap(),
                now,
                now
            ]))
    }
}

//ToDo: add query for moving topic to other table.
impl Handler<UpdateTopic> for DatabaseService {
    type Result = ResponseFuture<Topic, ResError>;

    fn handle(&mut self, msg: UpdateTopic, _: &mut Self::Context) -> Self::Result {
        let t = msg.0;
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
            return Box::new(ft_err(ResError::BadRequest));
        }

        let _ = write!(&mut query, " WHERE id={} ", t.id.unwrap());
        if let Some(s) = t.user_id {
            let _ = write!(&mut query, "AND user_id={} ", s);
        }
        query.push_str("RETURNING *");

        Box::new(self.simple_query_one(query.as_str()))
    }
}

impl Handler<GetTopics> for DatabaseService {
    type Result = ResponseFuture<(Vec<Topic>, Vec<u32>), ResError>;

    fn handle(&mut self, msg: GetTopics, _: &mut Self::Context) -> Self::Result {
        Box::new(self.get_topics_by_id_with_uid(msg.0))
    }
}


pub enum GetTopicsCache {
    Latest(u32, i64),
    Popular(u32, i64),
    PopularAll(i64),
    Ids(Vec<u32>),
}

impl Message for GetTopicsCache {
    type Result = Result<(Vec<Topic>, Vec<u32>), ResError>;
}

impl Handler<GetTopicsCache> for CacheService {
    type Result = ResponseFuture<(Vec<Topic>, Vec<u32>), ResError>;

    fn handle(&mut self, msg: GetTopicsCache, _: &mut Self::Context) -> Self::Result {
        match msg {
            GetTopicsCache::Popular(id, page) =>
                Box::new(self.get_cache_with_uids_from_list(&format!("category:{}:list_pop", id), page, "topic")),
            GetTopicsCache::PopularAll(page) =>
                Box::new(self.get_cache_with_uids_from_list("category:all:list_pop", page, "topic")),
            GetTopicsCache::Latest(id, page) =>
                Box::new(self.get_cache_with_uids_from_zrevrange(&format!("category:{}:topics_time", id), page, "topic")),
            GetTopicsCache::Ids(ids) =>
                Box::new(self.get_cache_with_uids_from_ids(ids, "topic"))
        }
    }
}

#[derive(Message)]
pub struct AddTopicCache(pub Topic);

impl Handler<AddTopicCache> for CacheService {
    type Result = ();

    fn handle(&mut self, msg: AddTopicCache, ctx: &mut Self::Context) -> Self::Result {
        ctx.spawn(self.add_topic_cache(msg.0).into_actor(self));
    }
}