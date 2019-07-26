use futures::Future;

use actix::prelude::*;
use chrono::NaiveDateTime;
use tokio_postgres::{Row, SimpleQueryRow, SimpleQueryMessage, Statement, Client, types::ToSql};

use crate::util::{hash, jwt};

use crate::model::{
    actors::{ErrorReportRecipient, DatabaseService},
    common::GetUserId,
    errors::{ResError, RepError},
    post::Post,
    user::{User, AuthRequest, AuthResponse},
    category::Category,
    topic::Topic,
    talk::Talk,
};
use crate::handler::messenger::ErrorReportMessage;

impl DatabaseService {
    pub fn query_one<T>(c: &mut Client, st: &Statement, p: &[&dyn ToSql], rep: Option<ErrorReportRecipient>) -> impl Future<Item=T, Error=ResError>
        where T: FromRow {
        c.query(st, p)
            .into_future()
            .map_err(move |(e, _)| {
                Self::send_err_rep(rep.as_ref());
                e
            })
            .from_err()
            .and_then(|(r, _)| match r {
                Some(row) => Ok(FromRow::from_row(&row)),
                None => Err(ResError::BadRequest)
            })
    }

    pub fn query_one_simple<T>(c: &mut Client, query: &str, rep: Option<ErrorReportRecipient>) -> impl Future<Item=T, Error=ResError>
        where T: FromSimpleRow {
        c.simple_query(&query)
            .into_future()
            .map_err(move |(e, _)| {
                Self::send_err_rep(rep.as_ref());
                e
            })
            .from_err()
            .and_then(|(opt, _)| match opt {
                Some(msg) => match msg {
                    SimpleQueryMessage::Row(row) => FromSimpleRow::from_row(&row),
                    _ => Err(ResError::InternalServerError)
                }
                None => Err(ResError::BadRequest)
            })
    }

    pub fn query_single_row<T>(c: &mut Client, query: &str, index: usize, rep: Option<ErrorReportRecipient>) -> impl Future<Item=T, Error=ResError>
        where T: std::str::FromStr {
        c.simple_query(query)
            .into_future()
            .map_err(move |(e, _)| {
                Self::send_err_rep(rep.as_ref());
                e
            })
            .from_err()
            .and_then(move |(opt, _)| match opt {
                Some(msg) => match msg {
                    SimpleQueryMessage::Row(row) => row
                        .get(index)
                        .ok_or(ResError::BadRequest)?
                        .parse::<T>()
                        .map_err(|_| ResError::ParseError),
                    _ => Err(ResError::InternalServerError)
                }
                None => Err(ResError::InternalServerError)
            })
    }

    pub fn query_multi_simple_with_id<T>(c: &mut Client, query: &str, rep: Option<ErrorReportRecipient>) -> impl Future<Item=(Vec<T>, Vec<u32>), Error=ResError>
        where T: GetUserId + FromSimpleRow {
        let vec = Vec::with_capacity(20);
        let ids: Vec<u32> = Vec::with_capacity(21);

        c.simple_query(&query)
            .map_err(move |e| {
                Self::send_err_rep(rep.as_ref());
                e
            })
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
                Ok::<(_, Vec<u32>), ResError>((vec, ids))
            })
    }

    pub fn query_multi_with_id<T>(c: &mut Client, st: &Statement, p: &[&dyn ToSql], rep: Option<ErrorReportRecipient>) -> impl Future<Item=(Vec<T>, Vec<u32>), Error=ResError>
        where T: FromRow {
        let vec = Vec::with_capacity(20);
        let ids = Vec::with_capacity(21);

        c.query(st, p)
            .map_err(move |e| {
                Self::send_err_rep(rep.as_ref());
                e
            })
            .from_err()
            .fold((vec, ids), move |(mut vec, mut ids), row| {
                ids.push(row.get(1));
                vec.push(FromRow::from_row(&row));
                Ok::<_, ResError>((vec, ids))
            })
    }

    pub fn query_multi_simple_no_limit<T>(
        c: &mut Client,
        query: &str,
        rep: Option<ErrorReportRecipient>,
    ) -> impl Future<Item=Vec<T>, Error=ResError>
        where T: FromSimpleRow {
        Self::query_multi_simple(c, query, Vec::new(), rep)
    }

    fn query_multi_simple<T>(
        c: &mut Client,
        query: &str,
        vec: Vec<T>,
        rep: Option<ErrorReportRecipient>,
    ) -> impl Future<Item=Vec<T>, Error=ResError>
        where T: FromSimpleRow {
        c.simple_query(&query)
            .map_err(move |e| {
                Self::send_err_rep(rep.as_ref());
                e
            })
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
                Ok::<_, ResError>(vec)
            })
    }

    pub fn query_multi_no_limit<T>(
        c: &mut Client,
        st: &Statement,
        p: &[&dyn ToSql],
        rep: Option<ErrorReportRecipient>,
    ) -> impl Future<Item=Vec<T>, Error=ResError>
        where T: FromRow {
        Self::query_multi(c, st, p, Vec::new(), rep)
    }
    pub fn query_multi_limit<T>(
        c: &mut Client,
        st: &Statement,
        p: &[&dyn ToSql],
        rep: Option<ErrorReportRecipient>,
    ) -> impl Future<Item=Vec<T>, Error=ResError>
        where T: FromRow {
        Self::query_multi(c, st, p, Vec::with_capacity(21), rep)
    }
    fn query_multi<T>(
        c: &mut Client,
        st: &Statement,
        p: &[&dyn ToSql],
        vec: Vec<T>,
        rep: Option<ErrorReportRecipient>,
    ) -> impl Future<Item=Vec<T>, Error=ResError>
        where T: FromRow {
        c.query(st, p)
            .map_err(move |e| {
                Self::send_err_rep(rep.as_ref());
                e
            })
            .from_err()
            .fold(vec, move |mut vec, row| {
                vec.push(FromRow::from_row(&row));
                Ok::<_, ResError>(vec)
            })
    }

    fn send_err_rep(rep: Option<&ErrorReportRecipient>) {
        if let Some(rep) = rep {
            let _ = rep.do_send(ErrorReportMessage(RepError::Database));
        }
    }
}

pub trait FromSimpleRow
    where Self: std::marker::Sized {
    fn from_row(row: &SimpleQueryRow) -> Result<Self, ResError>;
}

impl FromSimpleRow for Talk {
    fn from_row(row: &SimpleQueryRow) -> Result<Self, ResError> {
        Ok(Talk {
            id: row.get(0).map(|s| s.parse::<u32>()).unwrap()?,
            name: row.get(1).ok_or(ResError::InternalServerError)?.to_owned(),
            description: row.get(2).ok_or(ResError::InternalServerError)?.to_owned(),
            secret: row.get(3).ok_or(ResError::InternalServerError)?.to_owned(),
            privacy: row.get(4).map(|s| s.parse::<u32>()).unwrap()?,
            owner: row.get(5).map(|s| s.parse::<u32>()).unwrap()?,
            admin: vec![],
            users: vec![],
        })
    }
}

impl FromSimpleRow for User {
    fn from_row(row: &SimpleQueryRow) -> Result<Self, ResError> {
        Ok(User {
            id: row.get(0).map(|s| s.parse::<u32>()).unwrap()?,
            username: row.get(1).ok_or(ResError::InternalServerError)?.to_owned(),
            email: row.get(2).ok_or(ResError::InternalServerError)?.to_owned(),
            hashed_password: row.get(3).ok_or(ResError::InternalServerError)?.to_owned(),
            avatar_url: row.get(4).ok_or(ResError::InternalServerError)?.to_owned(),
            signature: row.get(5).ok_or(ResError::InternalServerError)?.to_owned(),
            created_at: row.get(6).map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")).unwrap()?,
            privilege: row.get(7).map(|s| s.parse::<u32>()).unwrap()?,
            show_email: if row.get(8) == Some("f") { false } else { true },
            online_status: None,
            last_online: None,
        })
    }
}

impl FromSimpleRow for Post {
    fn from_row(row: &SimpleQueryRow) -> Result<Self, ResError> {
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
            post_content: row.get(5).ok_or(ResError::InternalServerError)?.to_owned(),
            created_at: row.get(6).map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")).unwrap()?,
            updated_at: row.get(7).map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")).unwrap()?,
            last_reply_time: None,
            is_locked: if row.get(8) == Some("f") { false } else { true },
            reply_count: None,
        })
    }
}

impl FromSimpleRow for Topic {
    fn from_row(row: &SimpleQueryRow) -> Result<Self, ResError> {
        Ok(Topic {
            id: row.get(0).map(|s| s.parse::<u32>()).unwrap()?,
            user_id: row.get(1).map(|s| s.parse::<u32>()).unwrap()?,
            category_id: row.get(2).map(|s| s.parse::<u32>()).unwrap()?,
            title: row.get(3).ok_or(ResError::InternalServerError)?.to_owned(),
            body: row.get(4).ok_or(ResError::InternalServerError)?.to_owned(),
            thumbnail: row.get(5).ok_or(ResError::InternalServerError)?.to_owned(),
            created_at: row.get(6).map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")).unwrap()?,
            updated_at: row.get(7).map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")).unwrap()?,
            last_reply_time: None,
            is_locked: if row.get(8) == Some("f") { false } else { true },
            reply_count: None,
        })
    }
}

impl FromSimpleRow for Category {
    fn from_row(row: &SimpleQueryRow) -> Result<Self, ResError> {
        Ok(Category {
            id: row.get(0).map(|s| s.parse::<u32>()).unwrap()?,
            name: row.get(1).ok_or(ResError::InternalServerError)?.to_owned(),
            thumbnail: row.get(2).ok_or(ResError::InternalServerError)?.to_owned(),
            topic_count: None,
            post_count: None,
            topic_count_new: None,
            post_count_new: None,
        })
    }
}

pub trait FromRow {
    fn from_row(row: &Row) -> Self;
}

impl FromRow for Talk {
    fn from_row(row: &Row) -> Self {
        Talk {
            id: row.get(0),
            name: row.get(1),
            description: row.get(2),
            secret: row.get(3),
            privacy: row.get(4),
            owner: row.get(5),
            admin: row.get(6),
            users: row.get(7),
        }
    }
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
            privilege: row.get(7),
            show_email: row.get(8),
            online_status: None,
            last_online: None,
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
            last_reply_time: None,
            is_locked: row.get(8),
            reply_count: None,
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
            last_reply_time: None,
            is_locked: row.get(8),
            reply_count: None,
        }
    }
}

pub fn simple_query(
    c: &mut Client,
    query: &str,
) -> impl Future<Item=Option<SimpleQueryMessage>, Error=ResError> {
    c.simple_query(query)
        .into_future()
        .from_err()
        .map(|(msg, _)| msg)
}

pub fn auth_response_from_msg(
    opt: &Option<SimpleQueryMessage>,
    pass: &str,
) -> Result<AuthResponse, ResError> {
    match opt {
        Some(msg) => match msg {
            SimpleQueryMessage::Row(row) => auth_response_from_simple_row(row, pass),
            _ => Err(ResError::InvalidUsername)
        }
        None => Err(ResError::InternalServerError)
    }
}

pub fn unique_username_email_check(
    opt: &Option<SimpleQueryMessage>,
    req: &AuthRequest,
) -> Result<(), ResError> {
    match opt {
        Some(msg) => match msg {
            SimpleQueryMessage::Row(row) => {
                let row = row.get(0).ok_or(ResError::InternalServerError)?;
                if row == &req.username {
                    Err(ResError::UsernameTaken)
                } else {
                    Err(ResError::EmailTaken)
                }
            }
            _ => Ok(())
        }
        None => Err(ResError::BadRequest)
    }
}

fn auth_response_from_simple_row(
    row: &SimpleQueryRow,
    pass: &str,
) -> Result<AuthResponse, ResError> {
    let hash = row.get(3).ok_or(ResError::InternalServerError)?;
    let _ = hash::verify_password(pass, hash)?;

    let user: User = FromSimpleRow::from_row(row)?;
    let token = jwt::JwtPayLoad::new(user.id, user.privilege).sign()?;

    Ok(AuthResponse { token, user })
}