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
use crate::handler::db::{query_one_with_id, query_multi_with_id, query_one_simple};

const LIMIT: i64 = 20;

pub struct AddTopic(pub TopicRequest, pub GlobalGuard);

pub struct UpdateTopic(pub TopicRequest);

pub enum GetTopicWithPosts {
    Oldest(u32, i64),
    Popular(u32, i64),
}

pub enum GetTopics {
    Latest(Vec<u32>, i64),
    Popular(Vec<u32>, i64),
    PopularAll(i64),
}

impl Message for AddTopic {
    type Result = Result<Topic, ServiceError>;
}

impl Message for UpdateTopic {
    type Result = Result<Topic, ServiceError>;
}

impl Message for GetTopicWithPosts {
    type Result = Result<(Topic, Vec<Post>, Vec<u32>), ServiceError>;
}

impl Message for GetTopics {
    type Result = Result<(Vec<Topic>, Vec<u32>), ServiceError>;
}

impl Handler<GetTopicWithPosts> for DatabaseService {
    type Result = ResponseFuture<(Topic, Vec<Post>, Vec<u32>), ServiceError>;

    fn handle(&mut self, msg: GetTopicWithPosts, _: &mut Self::Context) -> Self::Result {
        let (stp, tid, page) = match msg {
            GetTopicWithPosts::Popular(tid, page) =>
                (self.posts_popular.as_ref().unwrap(), tid, page),
            GetTopicWithPosts::Oldest(tid, page) =>
                (self.posts_old.as_ref().unwrap(), tid, page)
        };

        let c = self.db.as_mut().unwrap();
        let stt = self.topic_by_id.as_ref().unwrap();
        let offset = (page - 1) * LIMIT;

        let f = query_one_with_id(c, stt, &[&tid])
            .join(query_multi_with_id(c, stp, &[&tid, &offset]))
            .map(|((t, uid), (p, mut ids))| {
                ids.push(uid);
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

        let f = self.db
            .as_mut()
            .unwrap()
            .query(self.insert_topic.as_ref().unwrap(),
                   &[&id,
                       &t.user_id.unwrap(),
                       &t.category_id,
                       &t.thumbnail.unwrap(),
                       &t.title.unwrap(),
                       &t.body.unwrap(),
                       &now,
                       &now,
                       &now
                   ])
            .into_future()
            .from_err()
            .and_then(|(r, _)| match r {
                Some(row) => Ok(Topic {
                    id: row.get(0),
                    user_id: row.get(1),
                    category_id: row.get(2),
                    title: row.get(3),
                    body: row.get(4),
                    thumbnail: row.get(5),
                    created_at: row.get(6),
                    updated_at: row.get(7),
                    last_reply_time: row.get(8),
                    reply_count: row.get(9),
                    is_locked: row.get(10),
                }),
                None => Err(ServiceError::BadRequest)
            });

        Box::new(f)
    }
}

//ToDo: add query for moving topic to other table.
impl Handler<UpdateTopic> for DatabaseService {
    type Result = ResponseFuture<Topic, ServiceError>;

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

        Box::new(query_one_simple(self.db.as_mut().unwrap(), &query))
    }
}

//ToDo: add multiple category_ids query
impl Handler<GetTopics> for DatabaseService {
    type Result = ResponseFuture<(Vec<Topic>, Vec<u32>), ServiceError>;

    fn handle(&mut self, msg: GetTopics, _: &mut Self::Context) -> Self::Result {
        let q = match msg {
            GetTopics::Latest(cid, page) => {
                self.db
                    .as_mut()
                    .unwrap()
                    .query(self.topics_latest.as_ref().unwrap(),
                           &[&cid, &((page - 1) * 20)])
            }
            GetTopics::Popular(cid, page) => {
                let yesterday = Utc::now().timestamp() - 86400;
                let yesterday = NaiveDateTime::from_timestamp(yesterday, 0);

                self.db
                    .as_mut()
                    .unwrap()
                    .query(self.topics_popular.as_ref().unwrap(),
                           &[&cid, &yesterday, &((page - 1) * 20)])
            }
            GetTopics::PopularAll(page) => {
                let yesterday = Utc::now().timestamp() - 86400;
                let yesterday = NaiveDateTime::from_timestamp(yesterday, 0);

                self.db
                    .as_mut()
                    .unwrap()
                    .query(self.topics_popular_all.as_ref().unwrap(),
                           &[&yesterday, &((page - 1) * 20)])
            }
        };

        let t = Vec::with_capacity(20);
        let ids: Vec<u32> = Vec::with_capacity(20);
        let f = q
            .from_err()
            .fold((t, ids), move |(mut t, mut ids), row| {
                ids.push(row.get(1));
                t.push(Topic {
                    id: row.get(0),
                    user_id: row.get(1),
                    category_id: row.get(2),
                    title: row.get(3),
                    body: row.get(4),
                    thumbnail: row.get(5),
                    created_at: row.get(6),
                    updated_at: row.get(7),
                    last_reply_time: row.get(8),
                    reply_count: row.get(9),
                    is_locked: row.get(10),
                });
                Ok::<(Vec<Topic>, Vec<u32>), ServiceError>((t, ids))
            });

        Box::new(f)
    }
}