use std::fmt::Write;
use futures::{Future, future::err as ft_err};

use actix::prelude::*;

use crate::model::{
    actors::DatabaseService,
    topic::TopicRequest,
    common::GlobalGuard,
    errors::ServiceError,
    topic::Topic,
    post::Post,
};
use crate::handler::db::{query_topics, query_posts, query_topic};

const LIMIT: i64 = 20;

pub struct AddTopic(pub TopicRequest, pub GlobalGuard);

pub struct UpdateTopic(pub TopicRequest);

pub struct GetTopicWithPost(pub u32, pub u32, pub i64);

pub enum GetTopics {
    Latest(u32, i64),
    Popular(i64),
}

impl Message for AddTopic {
    type Result = Result<Topic, ServiceError>;
}

impl Message for UpdateTopic {
    type Result = Result<Vec<Topic>, ServiceError>;
}

impl Message for GetTopicWithPost {
    type Result = Result<(Vec<Topic>, Vec<Post>, Vec<u32>), ServiceError>;
}

impl Message for GetTopics {
    type Result = Result<(Vec<Topic>, Vec<u32>), ServiceError>;
}


impl Handler<GetTopicWithPost> for DatabaseService {
    type Result = ResponseFuture<(Vec<Topic>, Vec<Post>, Vec<u32>), ServiceError>;

    fn handle(&mut self, msg: GetTopicWithPost, _: &mut Self::Context) -> Self::Result {
        let cid = msg.0;
        let tid = msg.1;
        let page = msg.2;

        let topic = Vec::with_capacity(1);
        let posts = Vec::with_capacity(20);

        let queryt = format!("SELECT * FROM topics{} WHERE id = {}", cid, tid);

        let queryp =
            format!("SELECT * FROM posts{}
                   WHERE topic_id={}
                   ORDER BY id ASC
                   OFFSET {}
                   LIMIT {}", cid, tid, (page - 1) * 20, LIMIT);

        let ft =
            query_topics(self.db.as_mut().unwrap(), &queryt, topic, posts);
        let fp =
            query_posts(self.db.as_mut().unwrap(), &queryp);

        let f = ft
            .join(fp)
            .map(|((t, mut tids), (p, mut ids))| {
                if let Some(id) = tids.pop() {
                    ids.push(id);
                }
                ids.sort();
                ids.dedup();
                (t, p, ids)
            });

        Box::new(f)
    }
}

impl Handler<AddTopic> for DatabaseService {
    type Result = ResponseFuture<Topic, ServiceError>;

    fn handle(&mut self, msg: AddTopic, _: &mut Self::Context) -> Self::Result {
        let id = match msg.1.lock() {
            Ok(mut var) => var.next_tid(),
            Err(_) => return Box::new(ft_err(ServiceError::InternalServerError))
        };
        let t = msg.0;

        let query = format!(
            "INSERT INTO topics{}
            (id, user_id, category_id, thumbnail, title, body)
            VALUES ({}, {}, {}, '{}', '{}', '{}')
            RETURNING *",
            t.category_id,
            id,
            t.user_id.unwrap(),
            t.category_id,
            t.thumbnail.unwrap(),
            t.title.unwrap(),
            t.body.unwrap());

        Box::new(query_topic(self.db.as_mut().unwrap(), &query))
    }
}

//ToDo: add query for moving topic to other table.
impl Handler<UpdateTopic> for DatabaseService {
    type Result = ResponseFuture<Vec<Topic>, ServiceError>;

    fn handle(&mut self, msg: UpdateTopic, _: &mut Self::Context) -> Self::Result {
        let t = msg.0;
        let mut query = String::new();

        let _ = write!(&mut query, "UPDATE topics{} SET", t.category_id);

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
            return Box::new(ft_err(ServiceError::BadRequest));
        }

        let _ = write!(&mut query, " WHERE id={} ", t.id.unwrap());
        if let Some(s) = t.user_id {
            let _ = write!(&mut query, "AND user_id={} ", s);
        }
        query.push_str("RETURNING *");

        Box::new(query_topic(self.db.as_mut().unwrap(), &query).map(|t| vec![t]))
    }
}

impl Handler<GetTopics> for DatabaseService {
    type Result = ResponseFuture<(Vec<Topic>, Vec<u32>), ServiceError>;

    fn handle(&mut self, msg: GetTopics, ctx: &mut Self::Context) -> Self::Result {
        let topics = Vec::with_capacity(20);
        let ids: Vec<u32> = Vec::with_capacity(20);

        let query = match msg {
            GetTopics::Latest(id, page) => format!(
                "SELECT * FROM topics{}
                ORDER BY last_reply_time DESC
                OFFSET {}
                LIMIT 20", id, ((page - 1) * 20)),
            GetTopics::Popular(page) => "template".to_owned()
        };

        let f = query_topics(self.db.as_mut().unwrap(), &query, topics, ids);

        Box::new(f)
    }
}