use std::fmt::Write;

use futures::{Future, future, IntoFuture};

use actix::prelude::*;
use chrono::NaiveDateTime;
use tokio_postgres::{Row, SimpleQueryRow, SimpleQueryMessage, Statement, Client};

use crate::util::{hash, jwt};

use crate::model::{
    errors::ServiceError,
    actors::DatabaseService,
    post::Post,
    user::{User, AuthRequest, AuthResponse},
    category::Category,
    topic::{Topic, TopicRequest},
    common::GlobalGuard,
};

pub fn get_all_categories(
    c: &mut Client,
    st: &Statement,
    categories: Vec<Category>,
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


pub fn get_posts(
    c: &mut Client,
    st: &Statement,
    id: u32,
    page: i64,
) -> impl Future<Item=(Vec<Post>, Vec<u32>), Error=ServiceError> {
    let posts = Vec::with_capacity(20);
    let ids: Vec<u32> = Vec::with_capacity(20);
    c.query(st, &[&id, &((page - 1) * 20)])
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
}


pub fn get_users(
    c: &mut Client,
    st: &Statement,
    ids: Vec<u32>,
) -> impl Future<Item=Vec<User>, Error=ServiceError> {
    let users = Vec::with_capacity(21);
    c.query(st, &[&ids])
        .from_err()
        .fold(users, move |mut users, row| {
            users.push(User {
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
            });
            Ok::<_, ServiceError>(users)
        })
}

pub fn get_users_all(
    c: &mut Client,
    st: &Statement,
) -> impl Future<Item=Vec<User>, Error=ServiceError> {
    let users = Vec::new();
    c.query(st, &[])
        .from_err()
        .fold(users, move |mut users, row| {
            users.push(User {
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
            });
            Ok::<_, ServiceError>(users)
        })
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

pub fn get_topics_test(
    mut c: Client,
    query: &str,
    topics: Vec<Topic>,
    ids: Vec<u32>,
) -> impl Future<Item=(Client, Vec<Topic>, Vec<u32>), Error=ServiceError> {
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
            Ok((c, t, ids))
        })
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

pub fn single_row_from_msg<T>(
    index: usize,
    opt: &Option<SimpleQueryMessage>,
) -> Result<T, ServiceError>
    where T: std::str::FromStr {
    match opt {
        Some(msg) => match msg {
            SimpleQueryMessage::Row(row) => row
                .get(index)
                .map(|s| s.parse::<T>())
                .unwrap()
                .map_err(|_| ServiceError::PARSEINT),
            _ => Err(ServiceError::InternalServerError)
        }
        None => Err(ServiceError::InternalServerError)
    }
}


pub fn category_from_msg(
    opt: &Option<SimpleQueryMessage>
) -> Result<Category, ServiceError> {
    match opt {
        Some(msg) => match msg {
            SimpleQueryMessage::Row(row) => category_from_simple_row(row),
            _ => Err(ServiceError::InternalServerError)
        }
        None => Err(ServiceError::InternalServerError)
    }
}

pub fn user_from_msg(
    opt: &Option<SimpleQueryMessage>
) -> Result<User, ServiceError> {
    match opt {
        Some(msg) => match msg {
            SimpleQueryMessage::Row(row) => user_from_simple_row(row),
            _ => Err(ServiceError::InternalServerError)
        }
        None => Err(ServiceError::InternalServerError)
    }
}

pub fn topic_from_msg(
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

pub fn post_from_msg(
    opt: &Option<SimpleQueryMessage>
) -> Result<Post, ServiceError> {
    match opt {
        Some(msg) => match msg {
            SimpleQueryMessage::Row(row) => post_from_simple_row(row),
            _ => Err(ServiceError::BadRequest)
        },
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

    let user = user_from_simple_row(row)?;
    let token = jwt::JwtPayLoad::new(user.id, user.is_admin).sign()?;

    Ok(AuthResponse { token, user })
}

fn user_from_simple_row(
    row: &SimpleQueryRow
) -> Result<User, ServiceError> {
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

fn post_from_simple_row(
    row: &SimpleQueryRow
) -> Result<Post, ServiceError> {
    let post_id = match row.get(3) {
        Some(s) => s.parse::<u32>().ok(),
        None => None
    };
    Ok(Post {
        id: row.get(0).map(|s| s.parse::<u32>()).unwrap()?,
        user_id: row.get(1).map(|s| s.parse::<u32>()).unwrap()?,
        topic_id: row.get(2).map(|s| s.parse::<u32>()).unwrap()?,
        post_id,
        post_content: row.get(4).ok_or(ServiceError::InternalServerError)?.to_owned(),
        created_at: row.get(5).map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")).unwrap()?,
        updated_at: row.get(6).map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")).unwrap()?,
        last_reply_time: row.get(7).map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")).unwrap()?,
        reply_count: row.get(8).map(|s| s.parse::<i32>()).unwrap()?,
        is_locked: if row.get(9) == Some("f") { false } else { true },
    })
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