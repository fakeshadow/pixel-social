use std::fmt::Write;

use futures::{Future, future, IntoFuture};

use actix::prelude::*;
use chrono::NaiveDateTime;
use tokio_postgres::{Row, SimpleQueryRow, SimpleQueryMessage, Statement, Client, types::ToSql};

use crate::util::{hash, jwt};

use crate::model::{
    errors::ServiceError,
    actors::DatabaseService,
    post::Post,
    user::{User, AuthRequest, AuthResponse},
    category::Category,
    topic::{Topic, TopicRequest},
    common::GlobalGuard,
    talk::Talk,
};
use crate::model::common::GetUserId;


pub trait FromSimpleRow
    where Self: std::marker::Sized {
    fn from_row(row: &SimpleQueryRow) -> Result<Self, ServiceError>;
}

impl FromSimpleRow for Talk {
    fn from_row(row: &SimpleQueryRow) -> Result<Self, ServiceError> {
        Ok(Talk {
            id: row.get(0).map(|s| s.parse::<u32>()).unwrap()?,
            name: row.get(1).ok_or(ServiceError::InternalServerError)?.to_owned(),
            description: row.get(2).ok_or(ServiceError::InternalServerError)?.to_owned(),
            secret: row.get(3).ok_or(ServiceError::InternalServerError)?.to_owned(),
            owner: row.get(4).map(|s| s.parse::<u32>()).unwrap()?,
            admin: vec![],
            users: vec![],
        })
    }
}

impl FromSimpleRow for User {
    fn from_row(row: &SimpleQueryRow) -> Result<Self, ServiceError> {
        Ok(User {
            id: row.get(0).map(|s| s.parse::<u32>()).unwrap()?,
            username: row.get(1).ok_or(ServiceError::InternalServerError)?.to_owned(),
            email: row.get(2).ok_or(ServiceError::InternalServerError)?.to_owned(),
            hashed_password: row.get(3).ok_or(ServiceError::InternalServerError)?.to_owned(),
            avatar_url: row.get(4).ok_or(ServiceError::InternalServerError)?.to_owned(),
            signature: row.get(5).ok_or(ServiceError::InternalServerError)?.to_owned(),
            created_at: row.get(6).map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")).unwrap()?,
            updated_at: row.get(7).map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")).unwrap()?,
            is_admin: row.get(8).map(|s| s.parse::<u32>()).unwrap()?,
            blocked: if row.get(9) == Some("f") { false } else { true },
            show_email: if row.get(10) == Some("f") { false } else { true },
            show_created_at: if row.get(11) == Some("f") { false } else { true },
            show_updated_at: if row.get(12) == Some("f") { false } else { true },
        })
    }
}

impl FromSimpleRow for Post {
    fn from_row(row: &SimpleQueryRow) -> Result<Self, ServiceError> {
        let post_id = match row.get(4) {
            Some(s) => s.parse::<u32>().ok(),
            None => None
        };
        Ok(Post {
            id: row.get(0).map(|s| s.parse::<u32>()).unwrap()?,
            user_id: row.get(1).map(|s| s.parse::<u32>()).unwrap()?,
            topic_id: row.get(2).map(|s| s.parse::<u32>()).unwrap()?,
            category_id: row.get(3).map(|s| s.parse::<u32>()).unwrap()?,
            post_id,
            post_content: row.get(5).ok_or(ServiceError::InternalServerError)?.to_owned(),
            created_at: row.get(6).map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")).unwrap()?,
            updated_at: row.get(7).map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")).unwrap()?,
            last_reply_time: row.get(8).map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")).unwrap()?,
            reply_count: row.get(9).map(|s| s.parse::<i32>()).unwrap()?,
            is_locked: if row.get(10) == Some("f") { false } else { true },
        })
    }
}

impl FromSimpleRow for Topic {
    fn from_row(row: &SimpleQueryRow) -> Result<Self, ServiceError> {
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
}

impl FromSimpleRow for Category {
    fn from_row(row: &SimpleQueryRow) -> Result<Self, ServiceError> {
        Ok(Category {
            id: row.get(0).map(|s| s.parse::<u32>()).unwrap()?,
            name: row.get(1).ok_or(ServiceError::InternalServerError)?.to_owned(),
            topic_count: row.get(2).map(|s| s.parse::<i32>()).unwrap()?,
            post_count: row.get(3).map(|s| s.parse::<i32>()).unwrap()?,
            subscriber_count: row.get(4).map(|s| s.parse::<i32>()).unwrap()?,
            thumbnail: row.get(5).ok_or(ServiceError::InternalServerError)?.to_owned(),
        })
    }
}

pub fn query_one_simple<T>(
    c: &mut Client,
    query: &str,
) -> impl Future<Item=T, Error=ServiceError>
    where T: FromSimpleRow {
    simple_query(c, &query)
        .and_then(move |opt| match opt {
            Some(msg) => match msg {
                SimpleQueryMessage::Row(row) => FromSimpleRow::from_row(&row),
                _ => Err(ServiceError::InternalServerError)
            }
            None => Err(ServiceError::BadRequest)
        })
}

pub fn query_multi_simple_with_id<T>(
    c: &mut Client,
    query: &str,
) -> impl Future<Item=(Vec<T>, Vec<u32>), Error=ServiceError>
    where T: GetUserId + FromSimpleRow {
    let vec = Vec::with_capacity(20);
    let ids: Vec<u32> = Vec::with_capacity(21);
    c.simple_query(&query)
        .from_err()
        .fold((vec, ids), move |(mut vec, mut ids), row| {
            match row {
                SimpleQueryMessage::Row(row) => {
                    let res: Option<T> = FromSimpleRow::from_row(&row).ok();
                    if let Some(v) = res {
                        ids.push(v.get_user_id());
                        vec.push(v);
                    }
                }
                _ => ()
            }
            Ok::<(Vec<T>, Vec<u32>), ServiceError>((vec, ids))
        })
}

pub fn query_all_simple<T>(
    c: &mut Client,
    query: &str,
) -> impl Future<Item=Vec<T>, Error=ServiceError>
    where T: FromSimpleRow {
    let vec = Vec::new();
    c.simple_query(&query)
        .from_err()
        .fold(vec, move |mut vec, row| {
            match row {
                SimpleQueryMessage::Row(row) => {
                    let res: Option<T> = FromSimpleRow::from_row(&row).ok();
                    if let Some(v) = res {
                        vec.push(v);
                    }
                }
                _ => ()
            }
            Ok::<Vec<T>, ServiceError>(vec)
        })
}

pub trait FromRow {
    fn from_row(row: &Row) -> Self;
}

impl FromRow for User {
    fn from_row(row: &Row) -> Self {
        User {
            id: row.get(0),
            username: row.get(1),
            email: row.get(2),
            hashed_password: "1".to_owned(),
            avatar_url: row.get(4),
            signature: row.get(5),
            created_at: row.get(6),
            updated_at: row.get(7),
            is_admin: row.get(8),
            blocked: row.get(9),
            show_email: row.get(10),
            show_created_at: row.get(11),
            show_updated_at: row.get(12),
        }
    }
}

impl FromRow for Post {
    fn from_row(row: &Row) -> Self {
        Post {
            id: row.get(0),
            user_id: row.get(1),
            topic_id: row.get(2),
            category_id: row.get(3),
            post_id: row.get(4),
            post_content: row.get(5),
            created_at: row.get(6),
            updated_at: row.get(7),
            last_reply_time: row.get(8),
            reply_count: row.get(9),
            is_locked: row.get(10),
        }
    }
}

impl FromRow for Topic {
    fn from_row(row: &Row) -> Self {
        Topic {
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
        }
    }
}

pub fn query_one<T>(
    c: &mut Client,
    st: &Statement,
    p: &[&dyn ToSql],
) -> impl Future<Item=T, Error=ServiceError>
    where T: FromRow {
    c.query(st, p)
        .into_future()
        .from_err()
        .and_then(|(r, _)| match r {
            Some(row) => Ok(FromRow::from_row(&row)),
            None => Err(ServiceError::BadRequest)
        })
}

pub fn query_one_with_id<T>(
    c: &mut Client,
    st: &Statement,
    p: &[&dyn ToSql],
) -> impl Future<Item=(T, u32), Error=ServiceError>
    where T: FromRow {
    c.query(st, p)
        .into_future()
        .from_err()
        .and_then(|(r, _)| match r {
            Some(row) => Ok((FromRow::from_row(&row), row.get(1))),
            None => Err(ServiceError::BadRequest)
        })
}

pub fn query_multi<T>(
    c: &mut Client,
    st: &Statement,
    p: &[&dyn ToSql],
) -> impl Future<Item=Vec<T>, Error=ServiceError>
    where T: FromRow {
    let vec = Vec::with_capacity(21);
    c.query(st, p)
        .from_err()
        .fold(vec, move |mut vec, row| {
            vec.push(FromRow::from_row(&row));
            Ok::<_, ServiceError>(vec)
        })
}

pub fn query_multi_with_id<T>(
    c: &mut Client,
    st: &Statement,
    p: &[&dyn ToSql],
) -> impl Future<Item=(Vec<T>, Vec<u32>), Error=ServiceError>
    where T: FromRow {
    let vec = Vec::with_capacity(20);
    let ids = Vec::with_capacity(21);
    c.query(st, p)
        .from_err()
        .fold((vec, ids), move |(mut vec, mut ids), row| {
            ids.push(row.get(1));
            vec.push(FromRow::from_row(&row));
            Ok::<_, ServiceError>((vec, ids))
        })
}

pub fn query_all<T>(
    c: &mut Client,
    st: &Statement,
    p: &[&dyn ToSql],
) -> impl Future<Item=Vec<T>, Error=ServiceError>
    where T: FromRow {
    let vec = Vec::new();
    c.query(st, p)
        .from_err()
        .fold(vec, move |mut vec, row| {
            vec.push(FromRow::from_row(&row));
            Ok::<_, ServiceError>(vec)
        })
}

pub fn simple_query(
    c: &mut Client,
    query: &str,
) -> impl Future<Item=Option<SimpleQueryMessage>, Error=ServiceError> {
    c.simple_query(query)
        .into_future()
        .from_err()
        .map(|(msg, _)| msg)
}

pub fn query_single_row<T>(
    c: &mut Client,
    query: &str,
    index: usize,
) -> impl Future<Item=T, Error=ServiceError>
    where T: std::str::FromStr {
    simple_query(c, query)
        .and_then(move |msg| match msg {
            Some(msg) => match msg {
                SimpleQueryMessage::Row(row) => row
                    .get(index)
                    .ok_or(ServiceError::BadRequest)?
                    .parse::<T>()
                    .map_err(|_|ServiceError::PARSEINT),
                _ => Err(ServiceError::InternalServerError)
            }
            None => Err(ServiceError::InternalServerError)
        })
}

pub fn talk_from_msg(
    opt: &Option<SimpleQueryMessage>
) -> Result<Talk, ServiceError> {
    match opt {
        Some(msg) => match msg {
            SimpleQueryMessage::Row(row) => FromSimpleRow::from_row(row),
            _ => Err(ServiceError::InternalServerError)
        }
        None => Err(ServiceError::InternalServerError)
    }
}

pub fn auth_response_from_msg(
    opt: &Option<SimpleQueryMessage>,
    pass: &str,
) -> Result<AuthResponse, ServiceError> {
    match opt {
        Some(msg) => match msg {
            SimpleQueryMessage::Row(row) => auth_response_from_simple_row(row, pass),
            _ => Err(ServiceError::InvalidUsername)
        }
        None => Err(ServiceError::InternalServerError)
    }
}

pub fn unique_username_email_check(
    opt: &Option<SimpleQueryMessage>,
    req: AuthRequest,
) -> Result<AuthRequest, ServiceError> {
    match opt {
        Some(msg) => match msg {
            SimpleQueryMessage::Row(row) => {
                let row = row.get(0).ok_or(ServiceError::InternalServerError)?;
                if row == &req.username {
                    Err(ServiceError::UsernameTaken)
                } else {
                    Err(ServiceError::EmailTaken)
                }
            }
            _ => Ok(req)
        }
        None => Err(ServiceError::BadRequest)
    }
}

fn auth_response_from_simple_row(
    row: &SimpleQueryRow,
    pass: &str,
) -> Result<AuthResponse, ServiceError> {
    let hash = row.get(3).ok_or(ServiceError::InternalServerError)?;
    let _ = hash::verify_password(pass, hash)?;

    let user: User = FromSimpleRow::from_row(row)?;
    let token = jwt::JwtPayLoad::new(user.id, user.is_admin).sign()?;

    Ok(AuthResponse { token, user })
}