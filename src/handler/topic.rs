use std::fmt::Write;
use futures::{Future, future::err as ft_err};

use actix::prelude::*;
use chrono::{NaiveDateTime, Utc};

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

pub enum GetTopicWithPosts {
    Oldest(u32, i64),
    Popular(u32, i64),
}

pub enum GetTopics {
    Latest(u32, i64),
    Popular(u32, i64),
    PopularAll(i64),
}

impl Message for AddTopic {
    type Result = Result<Topic, ServiceError>;
}

impl Message for UpdateTopic {
    type Result = Result<Vec<Topic>, ServiceError>;
}

impl Message for GetTopicWithPosts {
    type Result = Result<(Vec<Topic>, Vec<Post>, Vec<u32>), ServiceError>;
}

impl Message for GetTopics {
    type Result = Result<(Vec<Topic>, Vec<u32>), ServiceError>;
}


impl Handler<GetTopicWithPosts> for DatabaseService {
    type Result = ResponseFuture<(Vec<Topic>, Vec<Post>, Vec<u32>), ServiceError>;

    fn handle(&mut self, msg: GetTopicWithPosts, _: &mut Self::Context) -> Self::Result {
        let (tid, queryp) = match msg {
            GetTopicWithPosts::Popular(tid, page) => {
                let queryp =
                    format!("SELECT * FROM posts
                   WHERE topic_id = {}
                   ORDER BY reply_count DESC, id ASC
                   OFFSET {}
                   LIMIT {}", tid, (page - 1) * LIMIT, LIMIT);
                (tid, queryp)
            }
            GetTopicWithPosts::Oldest(tid, page) => {
                let queryp =
                    format!("SELECT * FROM posts
                   WHERE topic_id = {}
                   ORDER BY id ASC
                   OFFSET {}
                   LIMIT {}", tid, (page - 1) * LIMIT, LIMIT);
                (tid, queryp)
            }
        };

        let queryt = format!("SELECT * FROM topics WHERE id = {}", tid);

        let ft =
            query_topics(self.db.as_mut().unwrap(), &queryt);
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
        let now = Utc::now().naive_local();

        let query = format!(
            "INSERT INTO topics
            (id, user_id, category_id, thumbnail, title, body, created_at, updated_at, last_reply_time)
            VALUES ({}, {}, {}, '{}', '{}', '{}', '{}', '{}', '{}')
            RETURNING *",
            id,
            t.user_id.unwrap(),
            t.category_id,
            t.thumbnail.unwrap(),
            t.title.unwrap(),
            t.body.unwrap(),
            &now,
            &now,
            &now
        );

        Box::new(query_topic(self.db.as_mut().unwrap(), &query))
    }
}

//ToDo: add query for moving topic to other table.
impl Handler<UpdateTopic> for DatabaseService {
    type Result = ResponseFuture<Vec<Topic>, ServiceError>;

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

//ToDo: add multiple category_ids query
impl Handler<GetTopics> for DatabaseService {
    type Result = ResponseFuture<(Vec<Topic>, Vec<u32>), ServiceError>;

    fn handle(&mut self, msg: GetTopics, _: &mut Self::Context) -> Self::Result {
        let query = match msg {
            GetTopics::Latest(id, page) => format!(
                "SELECT * FROM topics
                WHERE category_id = {}
                ORDER BY last_reply_time DESC
                OFFSET {}
                LIMIT 20", id, ((page - 1) * 20)),
            GetTopics::Popular(cid, page) => {
                let yesterday = Utc::now().timestamp() - 86400;
                let yesterday = NaiveDateTime::from_timestamp(yesterday, 0);

                format!(
                    "SELECT * FROM topics
                WHERE last_reply_time > '{}' AND category_id = {}
                ORDER BY reply_count DESC
                OFFSET {}
                LIMIT 20", &yesterday, cid, ((page - 1) * 20))
            }
            GetTopics::PopularAll(page) => {
                let yesterday = Utc::now().timestamp() - 86400;
                let yesterday = NaiveDateTime::from_timestamp(yesterday, 0);

                format!(
                    "SELECT * FROM topics
                WHERE last_reply_time > '{}'
                ORDER BY reply_count DESC
                OFFSET {}
                LIMIT 20", &yesterday, ((page - 1) * 20))
            }
        };

        let f = query_topics(self.db.as_mut().unwrap(), &query);

        Box::new(f)
    }
}