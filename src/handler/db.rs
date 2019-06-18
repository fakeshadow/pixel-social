use std::fmt::Write;

use futures::{Future, future, IntoFuture};

use actix::prelude::*;
use chrono::NaiveDateTime;
use tokio_postgres::{Row, SimpleQueryRow, SimpleQueryMessage, Statement, Client};

use crate::util::{hash, jwt};

use crate::model::{
    errors::ServiceError,
    actors::PostgresConnection,
    post::Post,
    category::Category,
    topic::{Topic, TopicRequest},
    common::GlobalGuard,
};

pub enum GetTopics {
    Latest(u32, i64),
    Popular(i64),
}

impl Message for GetTopics {
    type Result = Result<(Vec<Topic>, Vec<u32>), ServiceError>;
}

impl Handler<GetTopics> for PostgresConnection {
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

        Box::new(get_topics(
            self.db.as_mut().unwrap(),
            &query, topics,
            ids,
        ))
    }
}

pub fn get_topics(
    c: &mut Client,
    query: &str,
    topics: Vec<Topic>,
    ids: Vec<u32>,
) -> impl Future<Item=(Vec<Topic>, Vec<u32>), Error=ServiceError> {
    c.simple_query(query)
        .from_err()
        .fold((topics, ids), move |(mut topics, mut ids), row| {
            if let Some(t) = topic_from_msg(&Some(row)).ok() {
                ids.push(t.user_id);
                topics.push(t);
            }
            Ok::<_, ServiceError>((topics, ids))
        })
        .and_then(|(t, mut ids)| {
            ids.sort();
            ids.dedup();
            Ok((t, ids))
        })
}


pub struct GetCategories;

impl Message for GetCategories {
    type Result = Result<Vec<Category>, ServiceError>;
}

impl Handler<GetCategories> for PostgresConnection {
    type Result = ResponseFuture<Vec<Category>, ServiceError>;

    fn handle(&mut self, _: GetCategories, _: &mut Self::Context) -> Self::Result {
        let categories = Vec::new();
        Box::new(get_all_categories(
            self.db.as_mut().unwrap(),
            self.categories.as_ref().unwrap(),
            categories))
    }
}

pub fn get_all_categories(
    c: &mut Client,
    st: &Statement,
    mut categories: Vec<Category>,
) -> impl Future<Item=Vec<Category>, Error=ServiceError> {
    c.query(st, &[])
        .from_err()
        .fold(categories, move |mut categories, row| {
            categories.push(Category {
                id: row.get(0),
                name: row.get(1),
                topic_count: row.get(2),
                post_count: row.get(3),
                subscriber_count: row.get(4),
                thumbnail: row.get(5),
            });
            Ok::<_, ServiceError>(categories)
        })
}

pub struct GetTopic(pub u32);

impl Message for GetTopic {
    type Result = Result<Vec<Topic>, ServiceError>;
}

impl Handler<GetTopic> for PostgresConnection {
    type Result = ResponseFuture<Vec<Topic>, ServiceError>;

    fn handle(&mut self, msg: GetTopic, _: &mut Self::Context) -> Self::Result {
        Box::new(self.db
            .as_mut()
            .unwrap()
            .query(self.posts_by_tid.as_ref().unwrap(), &[&vec![msg.0]])
            .into_future()
            .map_err(|(e, _)| e)
            .from_err()
            .and_then(|(row, _)|
                topic_from_row(row).map(|t| vec![t]))
        )
    }
}

pub struct GetPosts(pub u32, pub i64);

impl Message for GetPosts {
    type Result = Result<(Vec<Post>, Vec<u32>), ServiceError>;
}

impl Handler<GetPosts> for PostgresConnection {
    type Result = ResponseFuture<(Vec<Post>, Vec<u32>), ServiceError>;

    fn handle(&mut self, msg: GetPosts, _: &mut Self::Context) -> Self::Result {
        let posts = Vec::with_capacity(20);
        let ids: Vec<u32> = Vec::with_capacity(20);
        Box::new(self.db
            .as_mut()
            .unwrap()
            .query(self.posts_by_tid.as_ref().unwrap(), &[&msg.0, &((msg.1 - 1) * 20)])
            .from_err()
            .fold((posts, ids), move |(mut posts, mut ids), row| {
                ids.push(row.get(1));
                posts.push(Post {
                    id: row.get(0),
                    user_id: row.get(1),
                    topic_id: row.get(2),
                    post_id: row.get(3),
                    post_content: row.get(4),
                    created_at: row.get(5),
                    updated_at: row.get(6),
                    last_reply_time: row.get(7),
                    reply_count: row.get(8),
                    is_locked: row.get(9),
                });
                Ok::<_, ServiceError>((posts, ids))
            })
        )
    }
}

pub struct AddTopic(pub TopicRequest, pub GlobalGuard);

impl Message for AddTopic {
    type Result = Result<(Topic), ServiceError>;
}

impl Handler<AddTopic> for PostgresConnection {
    type Result = ResponseFuture<(Topic), ServiceError>;

    fn handle(&mut self, msg: AddTopic, _: &mut Self::Context) -> Self::Result {
        let id = match msg.1.lock() {
            Ok(mut var) => var.next_tid(),
            Err(_) => return Box::new(future::err(ServiceError::InternalServerError))
        };
        let t = match msg.0.make_topic(&id) {
            Ok(t) => t,
            Err(e) => return Box::new(future::err(e))
        };

        let query = format!(
            "INSERT INTO topics{}
            (id, user_id, category_id, thumbnail, title, body)
            VALUES ('{}', '{}', '{}', '{}', '{}', '{}')
            RETURNING *",
            t.category_id, t.id, t.user_id, t.category_id, t.thumbnail, t.title, t.body);

        Box::new(simple_query(self.db.as_mut().unwrap(), &query)
            .and_then(|msg| topic_from_msg(&msg)))
    }
}

pub struct UpdateTopic(pub TopicRequest);

impl Message for UpdateTopic {
    type Result = Result<Vec<Topic>, ServiceError>;
}

impl Handler<UpdateTopic> for PostgresConnection {
    type Result = ResponseFuture<Vec<Topic>, ServiceError>;

    fn handle(&mut self, msg: UpdateTopic, _: &mut Self::Context) -> Self::Result {
        let t = match msg.0.make_update() {
            Ok(t) => t,
            Err(e) => return Box::new(future::err(e))
        };

        let mut query = String::new();
        query.push_str("UPDATE topics SET");

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
            let _ = write!(&mut query, " is_locked='{}',", s);
        }
        if let Some(s) = t.category_id {
            let _ = write!(&mut query, " category_id='{}',", s);
        }
        // update update_at or return err as the query is empty.
        if query.ends_with(",") {
            let _ = write!(&mut query, " updated_at=DEFAULT");
        } else {
            return Box::new(future::err(ServiceError::BadRequest));
        }

        let _ = write!(&mut query, " WHERE id='{}' ", t.id);
        if let Some(s) = t.user_id {
            let _ = write!(&mut query, "AND user_id='{}' ", s);
        }
        query.push_str("RETURNING *");

        Box::new(simple_query(self.db.as_mut().unwrap(), &query)
            .and_then(|msg| topic_from_msg(&msg).map(|t| vec![t])))
    }
}

// helper functions
pub fn simple_query(
    c: &mut Client,
    query: &str,
) -> impl Future<Item=Option<SimpleQueryMessage>, Error=ServiceError> {
    c.simple_query(query)
        .into_future()
        .map_err(|(e, _)| e)
        .from_err()
        .and_then(|(msg, _)| Ok(msg))
}

fn topic_from_msg(
    opt: &Option<SimpleQueryMessage>
) -> Result<Topic, ServiceError> {
    match opt {
        Some(msg) => match msg {
            SimpleQueryMessage::Row(row) => topic_from_simple_row(row),
            _ => Err(ServiceError::BadRequest)
        },
        None => Err(ServiceError::InternalServerError)
    }
}

fn topic_from_simple_row(
    row: &SimpleQueryRow
) -> Result<Topic, ServiceError> {
    Ok(Topic {
        id: row.get(0).map(|s| s.parse::<u32>()).unwrap()?,
        user_id: row.get(1).map(|s| s.parse::<u32>()).unwrap()?,
        category_id: row.get(2).map(|s| s.parse::<u32>()).unwrap()?,
        title: row.get(3).ok_or(ServiceError::InternalServerError)?.to_owned(),
        body: row.get(4).ok_or(ServiceError::InternalServerError)?.to_owned(),
        thumbnail: row.get(5).ok_or(ServiceError::InternalServerError)?.to_owned(),
        created_at: row.get(6).map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")).unwrap()?,
        updated_at: row.get(7).map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")).unwrap()?,
        last_reply_time: row.get(8).map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")).unwrap()?,
        reply_count: row.get(9).map(|s| s.parse::<i32>()).unwrap()?,
        is_locked: if row.get(10) == Some("f") { false } else { true },
    })
}

fn category_from_simple_row(
    row: &SimpleQueryRow
) -> Result<Category, ServiceError> {
    Ok(Category {
        id: row.get(0).map(|s| s.parse::<u32>()).unwrap()?,
        name: row.get(1).ok_or(ServiceError::InternalServerError)?.to_owned(),
        topic_count: row.get(2).map(|s| s.parse::<i32>()).unwrap()?,
        post_count: row.get(3).map(|s| s.parse::<i32>()).unwrap()?,
        subscriber_count: row.get(4).map(|s| s.parse::<i32>()).unwrap()?,
        thumbnail: row.get(5).ok_or(ServiceError::InternalServerError)?.to_owned(),
    })
}

fn topic_from_row(
    row: Option<Row>
) -> Result<Topic, ServiceError> {
    match row {
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
        None => Err(ServiceError::InternalServerError)
    }
}

fn category_from_row(
    row: Option<Row>
) -> Result<Category, ServiceError> {
    match row {
        Some(row) => Ok(Category {
            id: row.get(0),
            name: row.get(1),
            topic_count: row.get(2),
            post_count: row.get(3),
            subscriber_count: row.get(4),
            thumbnail: row.get(5),
        }),
        None => Err(ServiceError::InternalServerError)
    }
}