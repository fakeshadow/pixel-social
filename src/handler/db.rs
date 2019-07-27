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
    talk::Talk,
};

impl TalkService {
    pub fn query_one(
        c: &mut Client,
        st: &Statement,
        p: &[&dyn ToSql],
    ) -> impl Future<Item=(), Error=ResError> {
        query_one_handler(c, st, p, None).map(|_|())
    }

    pub fn simple_query_talk(
        &mut self,
        q: &str,
    ) -> impl Future<Item=Talk, Error=ResError> {
        simple_query_one_handler(
            self.db.as_mut().unwrap(),
            q,
            None)
            .map(Talk::from)
    }

    pub fn simple_query_single_row<T>(
        &mut self,
        q: &str,
        index: usize,
    ) -> impl Future<Item=T, Error=ResError>
        where T: std::str::FromStr {
        simple_query_single_row_handler(
            self.db.as_mut().unwrap(),
            q,
            index,
            None)
    }
}


impl DatabaseService {
    pub fn query_one<T>(
        c: &mut Client,
        st: &Statement,
        p: &[&dyn ToSql],
        rep: Option<ErrorReportRecipient>,
    ) -> impl Future<Item=T, Error=ResError>
        where T: From<Row> {
        query_one_handler(c, st, p, rep).map(T::from)
    }

    pub fn simple_query_one<T>(
        &mut self,
        q: &str,
    ) -> impl Future<Item=T, Error=ResError>
        where T: From<SimpleQueryRow> {
        simple_query_one_handler(
            self.db.as_mut().unwrap(),
            q,
            self.error_reprot.as_ref().map(Clone::clone))
            .map(T::from)
    }

    pub fn query_multi<T>(
        c: &mut Client,
        st: &Statement,
        ids: &Vec<u32>,
        rep: Option<ErrorReportRecipient>,
    ) -> impl Future<Item=Vec<T>, Error=ResError>
        where T: From<Row> + GetSelfId {
        query_stream_handler(c, st, &[&ids], rep)
            .fold(Vec::with_capacity(ids.len()), move |mut vec, row| {
                vec.push(T::from(row));
                Ok::<_, ResError>(vec)
            })
    }

    pub fn query_multi_with_uid<T>(
        c: &mut Client,
        st: &Statement,
        ids_org: Vec<u32>,
        rep: Option<ErrorReportRecipient>,
    ) -> impl Future<Item=(Vec<T>, Vec<u32>), Error=ResError>
        where T: From<Row> + GetSelfId {
        let len = ids_org.len();
        let vec = Vec::with_capacity(len);
        let ids = Vec::with_capacity(len);

        query_stream_handler(c, st, &[&ids_org], rep)
            .fold((vec, ids), move |(mut vec, mut ids), row| {
                let user_id: u32 = row.get(1);
                ids.push(user_id);
                vec.push(T::from(row));
                Ok::<_, ResError>((vec, ids))
            })
            .map(move |(p, uids)| (sort_vec(p, ids_org), uids))
    }

    pub fn simple_query_multi_no_limit<T>(
        &mut self,
        q: &str,
    ) -> impl Future<Item=Vec<T>, Error=ResError>
        where T: From<SimpleQueryRow> {
        simple_query_multi_handler(
            self.db.as_mut().unwrap(),
            q,
            self.error_reprot.as_ref().map(Clone::clone),
            Vec::new())
    }

    pub fn simple_query_single_row<T>(
        &mut self,
        q: &str,
        i: usize,
    ) -> impl Future<Item=T, Error=ResError>
        where T: std::str::FromStr {
        simple_query_single_row_handler(
            self.db.as_mut().unwrap(),
            q,
            i,
            self.error_reprot.as_ref().map(Clone::clone))
    }

    pub fn simple_query_row(
        &mut self,
        q: &str,
    ) -> impl Future<Item=SimpleQueryRow, Error=ResError> {
        simple_query_one_handler(
            self.db.as_mut().unwrap(),
            q,
            self.error_reprot.as_ref().map(Clone::clone))
    }
}

pub fn load_all<T>(
    c: &mut Client,
    q: &str,
    rep: Option<ErrorReportRecipient>,
) -> impl Future<Item=Vec<T>, Error=ResError>
    where T: From<SimpleQueryRow> {
    simple_query_multi_handler(c, q, rep, Vec::new())
}

pub fn simple_query_single_row_handler<T>(
    c: &mut Client,
    query: &str,
    index: usize,
    rep: Option<ErrorReportRecipient>,
) -> impl Future<Item=T, Error=ResError>
    where T: std::str::FromStr {
    simple_query_one_handler(c, query, rep)
        .and_then(move |r| r
            .get(index)
            .ok_or(ResError::BadRequest)?
            .parse::<T>()
            .map_err(|_| ResError::ParseError))
}

fn simple_query_multi_handler<T>(
    c: &mut Client,
    q: &str,
    rep: Option<ErrorReportRecipient>,
    vec: Vec<T>,
) -> impl Future<Item=Vec<T>, Error=ResError>
    where T: From<SimpleQueryRow> {
    simple_query_stream_handler(c, q, rep)
        .fold(vec, move |mut vec, row| {
            if let SimpleQueryMessage::Row(row) = row {
                vec.push(T::from(row))
            }
            Ok::<_, ResError>(vec)
        })
}

fn query_one_handler(
    c: &mut Client,
    st: &Statement,
    p: &[&dyn ToSql],
    rep: Option<ErrorReportRecipient>,
) -> impl Future<Item=Row, Error=ResError> {
    query_stream_handler(c, st, p, rep)
        .into_future()
        .map_err(|(e, _)| e)
        .and_then(|(r, _)| r.ok_or(ResError::BadRequest))
}

pub fn simple_query_one_handler(
    c: &mut Client,
    query: &str,
    rep: Option<ErrorReportRecipient>,
) -> impl Future<Item=SimpleQueryRow, Error=ResError> {
    simple_query_stream_handler(c, query, rep)
        .into_future()
        .map_err(|(e, _)| e)
        .and_then(|(r, _)| match r {
            Some(msg) => match msg {
                SimpleQueryMessage::Row(row) => Ok(row),
                _ => Err(ResError::InternalServerError)
            }
            None => Err(ResError::BadRequest)
        })
}

fn query_stream_handler(
    c: &mut Client,
    st: &Statement,
    p: &[&dyn ToSql],
    rep: Option<ErrorReportRecipient>,
) -> impl futures::Stream<Item=Row, Error=ResError> {
    c.query(st, p)
        .map_err(move |e| {
            send_err_rep(rep.as_ref());
            e
        })
        .from_err()
}

fn simple_query_stream_handler(
    c: &mut Client,
    query: &str,
    rep: Option<ErrorReportRecipient>,
) -> impl futures::Stream<Item=SimpleQueryMessage, Error=ResError> {
    c.simple_query(&query)
        .map_err(move |e| {
            send_err_rep(rep.as_ref());
            e
        })
        .from_err()
}

fn send_err_rep(rep: Option<&ErrorReportRecipient>) {
    if let Some(rep) = rep {
        let _ = rep.do_send(crate::handler::messenger::ErrorReportMessage(RepError::Database));
    }
}

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

fn sort_vec<T>(mut v: Vec<T>, ids: Vec<u32>) -> Vec<T>
    where T: GetSelfId + From<Row> {
    let mut result = Vec::with_capacity(v.len());
    for i in 0..ids.len() {
        for j in 0..v.len() {
            if &ids[i] == v[j].self_id() {
                result.push(v.swap_remove(j));
                break;
            }
        }
    }
    result
}


pub fn auth_response_from_simple_row(
    row: SimpleQueryRow,
    pass: &str,
) -> Result<AuthResponse, ResError> {
    let hash = row.get(3).ok_or(ResError::InternalServerError)?;
    let _ = hash::verify_password(pass, hash)?;

    let user = User::from(row);
    let token = jwt::JwtPayLoad::new(user.id, user.privilege).sign()?;

    Ok(AuthResponse { token, user })
}

pub fn unique_username_email_check(
    r: &SimpleQueryRow,
    req: &AuthRequest,
) -> Result<(), ResError> {
    let r = r.get(0).ok_or(ResError::InternalServerError)?;
    if r == &req.username {
        Err(ResError::UsernameTaken)
    } else {
        Err(ResError::EmailTaken)
    }
}
