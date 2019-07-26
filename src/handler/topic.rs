use std::fmt::Write;
use futures::{Future, future::err as ft_err};

use actix::prelude::*;
use chrono::Utc;

use crate::model::{
    actors::DatabaseService,
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
        let now = &Utc::now().naive_local();

        Box::new(Self::query_one(
            self.db.as_mut().unwrap(),
            self.insert_topic.as_ref().unwrap(),
            &[&id,
                &t.user_id.unwrap(),
                &t.category_id,
                &t.thumbnail.unwrap(),
                &t.title.unwrap(),
                &t.body.unwrap(),
                now,
                now],
            self.error_reprot.as_ref().map(|r| r.clone())))
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

        Box::new(Self::query_one_simple(
            self.db.as_mut().unwrap(),
            &query,
            self.error_reprot.as_ref().map(|r| r.clone())))
    }
}

impl Handler<GetTopics> for DatabaseService {
    type Result = ResponseFuture<(Vec<Topic>, Vec<u32>), ResError>;

    fn handle(&mut self, msg: GetTopics, _: &mut Self::Context) -> Self::Result {
        Box::new(
            Self::query_multi_with_id(
                self.db.as_mut().unwrap(),
                self.topics_by_id.as_ref().unwrap(),
                &[&msg.0],
                self.error_reprot.as_ref().map(|r| r.clone()))
                .map(move |(mut t, uids): (Vec<Topic>, Vec<u32>)| {
                    let mut result = Vec::with_capacity(t.len());
                    for i in 0..msg.0.len() {
                        for j in 0..t.len() {
                            if msg.0[i] == t[j].id {
                                result.push(t.swap_remove(j));
                                break;
                            }
                        }
                    }
                    (result, uids)
                })
        )
    }
}