use std::convert::From;
use futures::Future;

use actix::prelude::*;
use chrono::NaiveDateTime;
use tokio_postgres::{Row, SimpleQueryRow, SimpleQueryMessage, Statement, Client, types::ToSql};

use crate::util::{hash, jwt};

use crate::model::{
    actors::{ErrorReportRecipient, DatabaseService, TalkService},
    common::GetSelfId,
    errors::{ResError, RepError},
    post::Post,
    user::{User, AuthRequest, AuthResponse},
    category::Category,
    topic::Topic,
    talk::{Talk, PublicMessage, PrivateMessage},
};

impl DatabaseService {
    pub fn unique_username_email_check(&mut self, q: &str, req: AuthRequest) -> Box<dyn Future<Item=AuthRequest, Error=ResError>> {
        Box::new(self
            .simple_query_row(q)
            .then(|r| {
                if let Some(r) = r.ok() {
                    if let Some(r) = r.get(0) {
                        if r == &req.username {
                            return Err(ResError::UsernameTaken);
                        } else {
                            return Err(ResError::EmailTaken);
                        }
                    }
                }
                Ok(req)
            }))
    }

    pub fn generate_auth_response(&mut self, q: &str, pass: String) -> Box<dyn Future<Item=AuthResponse, Error=ResError>> {
        Box::new(self
            .simple_query_row(q)
            .and_then(move |r| {
                let hash = r.get(3).ok_or(ResError::InternalServerError)?;
                let _ = hash::verify_password(pass.as_str(), hash)?;

                let user = User::from(r);
                let token = jwt::JwtPayLoad::new(user.id, user.privilege).sign()?;

                Ok(AuthResponse { token, user })
            })
        )
    }
}


pub trait SimpleQuery {
    fn simple_query_stream(
        &mut self,
        query: &str,
    ) -> Box<dyn futures::Stream<Item=SimpleQueryMessage, Error=ResError>> {
        let (c, rep) = self.get_client_and_report();
        Box::new(c
            .simple_query(&query)
            .map_err(move |e| {
                send_rep(rep.as_ref());
                e
            })
            .from_err())
    }
    fn get_client_and_report(&mut self) -> (&mut Client, Option<ErrorReportRecipient>);
}

impl SimpleQuery for DatabaseService {
    fn get_client_and_report(&mut self) -> (&mut Client, Option<ErrorReportRecipient>) {
        (self.db.as_mut().unwrap(), self.error_reprot.as_ref().map(Clone::clone))
    }
}

impl SimpleQuery for TalkService {
    fn get_client_and_report(&mut self) -> (&mut Client, Option<ErrorReportRecipient>) {
        (self.db.as_mut().unwrap(), None)
    }
}

pub trait Query {
    fn query_stream(
        c: &mut Client,
        st: &Statement,
        p: &[&dyn ToSql],
        rep: Option<ErrorReportRecipient>,
    ) -> Box<dyn futures::Stream<Item=Row, Error=ResError>> {
        Box::new(c
            .query(st, p)
            .map_err(move |e| {
                send_rep(rep.as_ref());
                e
            })
            .from_err())
    }
}

impl Query for DatabaseService {}

impl Query for TalkService {}


pub trait SimpleQueryOne
    where Self: SimpleQuery {
    fn simple_query_single_row<T>(&mut self, q: &str, i: usize) -> Box<dyn Future<Item=T, Error=ResError>>
        where T: std::str::FromStr + 'static {
        Box::new(self
            .simple_query_row(q)
            .and_then(move |r| r
                .get(i)
                .ok_or(ResError::BadRequest)?
                .parse::<T>()
                .map_err(|_| ResError::ParseError)))
    }

    fn simple_query_one<T>(&mut self, q: &str) -> Box<dyn Future<Item=T, Error=ResError>>
        where T: From<SimpleQueryRow> + 'static {
        Box::new(self
            .simple_query_row(q)
            .map(T::from))
    }

    fn simple_query_row(&mut self, q: &str) -> Box<dyn Future<Item=SimpleQueryRow, Error=ResError>> {
        Box::new(self
            .simple_query_stream(q)
            .into_future()
            .map_err(|(e, _)| e)
            .and_then(|(r, _)| match r {
                Some(m) => match m {
                    SimpleQueryMessage::Row(r) => Ok(r),
                    _ => Err(ResError::NoContent)
                }
                None => Err(ResError::BadRequest)
            }))
    }
}

impl SimpleQueryOne for TalkService {}

impl SimpleQueryOne for DatabaseService {}

pub trait SimpleQueryMulti
    where Self: SimpleQuery {
    fn simple_query_multi<T>(&mut self, q: &str, v: Vec<T>) -> Box<dyn Future<Item=Vec<T>, Error=ResError>>
        where T: From<SimpleQueryRow> + 'static {
        Box::new(self
            .simple_query_stream(q)
            .fold(v, move |mut v, r| {
                if let SimpleQueryMessage::Row(r) = r {
                    v.push(T::from(r))
                }
                Ok::<_, ResError>(v)
            }))
    }
}

impl SimpleQueryMulti for DatabaseService {}

pub trait QueryOne
    where Self: Query {
    fn query_one<T>(
        c: &mut Client,
        st: &Statement,
        p: &[&dyn ToSql],
        rep: Option<ErrorReportRecipient>,
    ) -> Box<dyn Future<Item=T, Error=ResError>>
        where T: From<Row> + 'static {
        Box::new(Self::query_stream(c, st, p, rep)
            .into_future()
            .map_err(|(e, _)| e)
            .and_then(|(r, _)| r.ok_or(ResError::BadRequest))
            .map(T::from))
    }
}

impl QueryOne for DatabaseService {}

impl QueryOne for TalkService {}

pub trait QueryMulti
    where Self: Query {
    fn query_multi<T>(
        c: &mut Client,
        st: &Statement,
        p: &[&dyn ToSql],
        vec: Vec<T>,
        rep: Option<ErrorReportRecipient>,
    ) -> Box<dyn Future<Item=Vec<T>, Error=ResError>>
        where T: From<Row> + 'static {
        Box::new(Self::query_stream(c, st, p, rep)
            .fold(vec, move |mut vec, row| {
                vec.push(T::from(row));
                Ok::<_, ResError>(vec)
            }))
    }
}

impl QueryMulti for DatabaseService {}

impl QueryMulti for TalkService {}

pub trait QueryMultiWithUids
    where Self: Query {
    fn query_multi_with_uid<T>(
        c: &mut Client,
        st: &Statement,
        ids_org: Vec<u32>,
        rep: Option<ErrorReportRecipient>,
    ) -> Box<dyn Future<Item=(Vec<T>, Vec<u32>), Error=ResError>>
        where T: From<Row> + GetSelfId + 'static {
        let len = ids_org.len();
        let vec = Vec::with_capacity(len);
        let ids = Vec::with_capacity(len);

        Box::new(Self::query_stream(c, st, &[&ids_org], rep)
            .fold((vec, ids), move |(mut vec, mut ids), row| {
                let user_id: u32 = row.get(1);
                ids.push(user_id);
                vec.push(T::from(row));
                Ok::<_, ResError>((vec, ids))
            })
            .map(move |(mut v, uids)| {
                let mut result = Vec::with_capacity(v.len());
                for i in 0..uids.len() {
                    for j in 0..v.len() {
                        if &uids[i] == v[j].self_id() {
                            result.push(v.swap_remove(j));
                            break;
                        }
                    }
                }
                (result, uids)
            }))
    }
}

impl QueryMultiWithUids for DatabaseService {}

impl From<Row> for User {
    fn from(row: Row) -> Self {
        User {
            id: row.get(0),
            username: row.get(1),
            email: row.get(2),
            hashed_password: "1".to_owned(),
            avatar_url: row.get(4),
            signature: row.get(5),
            created_at: row.get(6),
            privilege: row.get(7),
            show_email: row.get(8),
            online_status: None,
            last_online: None,
        }
    }
}

impl From<Row> for Topic {
    fn from(row: Row) -> Self {
        Topic {
            id: row.get(0),
            user_id: row.get(1),
            category_id: row.get(2),
            title: row.get(3),
            body: row.get(4),
            thumbnail: row.get(5),
            created_at: row.get(6),
            updated_at: row.get(7),
            last_reply_time: None,
            is_locked: row.get(8),
            reply_count: None,
        }
    }
}

impl From<Row> for Post {
    fn from(row: Row) -> Self {
        Post {
            id: row.get(0),
            user_id: row.get(1),
            topic_id: row.get(2),
            category_id: row.get(3),
            post_id: row.get(4),
            post_content: row.get(5),
            created_at: row.get(6),
            updated_at: row.get(7),
            last_reply_time: None,
            is_locked: row.get(8),
            reply_count: None,
        }
    }
}

impl From<Row> for Talk {
    fn from(row: Row) -> Self {
        Talk {
            id: row.get(0),
            name: row.get(1),
            description: row.get(2),
            secret: row.get(3),
            privacy: row.get(4),
            owner: row.get(5),
            admin: vec![],
            users: vec![],
        }
    }
}

impl From<Row> for PublicMessage {
    fn from(row: Row) -> Self {
        PublicMessage {
            talk_id: row.get(0),
            time: row.get(1),
            text: row.get(2),
        }
    }
}

impl From<Row> for PrivateMessage {
    fn from(row: Row) -> Self {
        PrivateMessage {
            user_id: row.get(0),
            time: row.get(2),
            text: row.get(3),
        }
    }
}

impl From<SimpleQueryRow> for Post {
    fn from(r: SimpleQueryRow) -> Self {
        let post_id = match r.get(4) {
            Some(s) => s.parse::<u32>().ok(),
            None => None
        };
        Post {
            id: r.get(0).unwrap().parse::<u32>().unwrap(),
            user_id: r.get(1).unwrap().parse::<u32>().unwrap(),
            topic_id: r.get(2).unwrap().parse::<u32>().unwrap(),
            category_id: r.get(3).unwrap().parse::<u32>().unwrap(),
            post_id,
            post_content: r.get(5).unwrap().to_owned(),
            created_at: NaiveDateTime::parse_from_str(r.get(6).unwrap(), "%Y-%m-%d %H:%M:%S%.f").unwrap(),
            updated_at: NaiveDateTime::parse_from_str(r.get(7).unwrap(), "%Y-%m-%d %H:%M:%S%.f").unwrap(),
            last_reply_time: None,
            is_locked: if r.get(8) == Some("f") { false } else { true },
            reply_count: None,
        }
    }
}

impl From<SimpleQueryRow> for Topic {
    fn from(r: SimpleQueryRow) -> Self {
        Topic {
            id: r.get(0).unwrap().parse::<u32>().unwrap(),
            user_id: r.get(1).unwrap().parse::<u32>().unwrap(),
            category_id: r.get(2).unwrap().parse::<u32>().unwrap(),
            title: r.get(3).unwrap().to_owned(),
            body: r.get(4).unwrap().to_owned(),
            thumbnail: r.get(5).unwrap().to_owned(),
            created_at: NaiveDateTime::parse_from_str(r.get(6).unwrap(), "%Y-%m-%d %H:%M:%S%.f").unwrap(),
            updated_at: NaiveDateTime::parse_from_str(r.get(7).unwrap(), "%Y-%m-%d %H:%M:%S%.f").unwrap(),
            last_reply_time: None,
            is_locked: if r.get(8) == Some("f") { false } else { true },
            reply_count: None,
        }
    }
}

impl From<SimpleQueryRow> for User {
    fn from(r: SimpleQueryRow) -> Self {
        User {
            id: r.get(0).unwrap().parse::<u32>().unwrap(),
            username: r.get(1).unwrap().to_owned(),
            email: r.get(2).unwrap().to_owned(),
            hashed_password: r.get(3).unwrap().to_owned(),
            avatar_url: r.get(4).unwrap().to_owned(),
            signature: r.get(5).unwrap().to_owned(),
            created_at: NaiveDateTime::parse_from_str(r.get(6).unwrap(), "%Y-%m-%d %H:%M:%S%.f").unwrap(),
            privilege: r.get(7).unwrap().parse::<u32>().unwrap(),
            show_email: if r.get(8) == Some("f") { false } else { true },
            online_status: None,
            last_online: None,
        }
    }
}

impl From<SimpleQueryRow> for Category {
    fn from(r: SimpleQueryRow) -> Self {
        Category {
            id: r.get(0).unwrap().parse::<u32>().unwrap(),
            name: r.get(1).unwrap().to_owned(),
            thumbnail: r.get(2).unwrap().to_owned(),
            topic_count: None,
            post_count: None,
            topic_count_new: None,
            post_count_new: None,
        }
    }
}

impl From<SimpleQueryRow> for Talk {
    fn from(r: SimpleQueryRow) -> Self {
        Talk {
            id: r.get(0).unwrap().parse::<u32>().unwrap(),
            name: r.get(1).unwrap().to_owned(),
            description: r.get(2).unwrap().to_owned(),
            secret: r.get(3).unwrap().to_owned(),
            privacy: r.get(4).unwrap().parse::<u32>().unwrap(),
            owner: r.get(5).unwrap().parse::<u32>().unwrap(),
            admin: vec![],
            users: vec![],
        }
    }
}

fn send_rep(rep: Option<&ErrorReportRecipient>) {
    if let Some(rep) = rep {
        let _ = rep.do_send(crate::handler::messenger::ErrorReportMessage(RepError::Database));
    }
}

// helper functions for build cache on startup
pub fn load_all<T>(
    c: &mut Client,
    q: &str,
) -> impl Future<Item=Vec<T>, Error=ResError>
    where T: From<SimpleQueryRow> {
    c.simple_query(&q)
        .from_err()
        .fold(Vec::new(), move |mut vec, row| {
            if let SimpleQueryMessage::Row(row) = row {
                vec.push(T::from(row))
            }
            Ok::<_, ResError>(vec)
        })
}

pub fn simple_query_single_row_handler<T>(
    c: &mut Client,
    query: &str,
    index: usize,
) -> impl Future<Item=T, Error=ResError>
    where T: std::str::FromStr {
    c.simple_query(&query)
        .from_err()
        .into_future()
        .map_err(|(e, _)| e)
        .and_then(move |(r, _)| match r {
            Some(msg) => match msg {
                SimpleQueryMessage::Row(row) => row
                    .get(index)
                    .ok_or(ResError::BadRequest)?
                    .parse::<T>()
                    .map_err(|_| ResError::ParseError),
                _ => Err(ResError::NoContent)
            }
            None => Err(ResError::BadRequest)
        })
}