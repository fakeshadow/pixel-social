use std::fmt::Write;

use futures::{Future, future, IntoFuture};

use actix::prelude::*;
use chrono::NaiveDateTime;
use tokio_postgres::{Row, SimpleQueryRow, SimpleQueryMessage};

use crate::util::{hash, jwt};

use crate::model::{
    errors::ServiceError,
    db::PostgresConnection,
    user::{AuthRequest, AuthResponse, User},
    post::Post,
    category::Category,
    topic::{Topic, TopicRequest},
    common::GlobalGuard,
};
use crate::model::common::Validator;

pub struct GetTopics(pub Vec<u32>, pub i64);

impl Message for GetTopics {
    type Result = Result<(Vec<Topic>, Vec<u32>), ServiceError>;
}

impl Handler<GetTopics> for PostgresConnection {
    type Result = ResponseFuture<(Vec<Topic>, Vec<u32>), ServiceError>;

    fn handle(&mut self, msg: GetTopics, _: &mut Self::Context) -> Self::Result {
        let topics = Vec::with_capacity(20);
        let ids: Vec<u32> = Vec::with_capacity(20);

        Box::new(self.db
            .as_mut()
            .unwrap()
            .query(self.topics_by_cid.as_ref().unwrap(), &[&msg.0, &((msg.1 - 1) * 20)])
            .from_err()
            .fold((topics, ids), move |(mut topics, mut ids), row| {
                ids.push(row.get(1));
                ids.sort();
                ids.dedup();
                topics.push(Topic {
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
                Ok::<_, ServiceError>((topics, ids))
            })
        )
    }
}

pub struct GetCategories;

impl Message for GetCategories {
    type Result = Result<Vec<Category>, ServiceError>;
}

impl Handler<GetCategories> for PostgresConnection {
    type Result = ResponseFuture<Vec<Category>, ServiceError>;

    fn handle(&mut self, _: GetCategories, _: &mut Self::Context) -> Self::Result {
        let categories = Vec::new();
        Box::new(self.db
            .as_mut()
            .unwrap()
            .query(self.categories.as_ref().unwrap(), &[])
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
            }))
    }
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
            .query(self.topics_by_id.as_ref().unwrap(), &[&vec![msg.0]])
            .into_future()
            .map_err(|(e, _)| e)
            .from_err()
            .and_then(|(row, _)| topic_from_row(row))
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

pub struct GetUsers(pub Vec<u32>);

impl Message for GetUsers {
    type Result = Result<Vec<User>, ServiceError>;
}

impl Handler<GetUsers> for PostgresConnection {
    type Result = ResponseFuture<Vec<User>, ServiceError>;

    fn handle(&mut self, msg: GetUsers, _: &mut Self::Context) -> Self::Result {
        let users = Vec::with_capacity(21);

        Box::new(self.db
            .as_mut()
            .unwrap()
            .query(self.users_by_id.as_ref().unwrap(), &[&msg.0])
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
        )
    }
}

pub struct AddTopic(pub TopicRequest, pub GlobalGuard);

impl Message for AddTopic {
    type Result = Result<Vec<Topic>, ServiceError>;
}

impl Handler<AddTopic> for PostgresConnection {
    type Result = ResponseFuture<Vec<Topic>, ServiceError>;

    fn handle(&mut self, msg: AddTopic, _: &mut Self::Context) -> Self::Result {
        let id = match msg.1.lock() {
            Ok(mut var) => var.next_tid(),
            Err(_) => return Box::new(future::err(ServiceError::InternalServerError))
        };
        let t = match msg.0.make_topic(&id) {
            Ok(t) => t,
            Err(e) => return Box::new(future::err(e))
        };

        Box::new(self.db
            .as_mut()
            .unwrap()
            .query(self.add_topic.as_ref().unwrap(),
                   &[&id, &t.user_id, &t.category_id, &t.thumbnail, &t.title, &t.body])
            .into_future()
            .map_err(|(e, _)| e)
            .from_err()
            .and_then(|(row, _)| topic_from_row(row))
        )
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

        Box::new(self.db
            .as_mut()
            .unwrap()
            .simple_query(&query)
            .into_future()
            .map_err(|(e, _)| e)
            .from_err()
            .and_then(|(r, _)| match r {
                Some(s) => match s {
                    SimpleQueryMessage::Row(row) => topic_from_simple_row(row),
                    _ => return Err(ServiceError::InternalServerError)
                }
                None => return Err(ServiceError::InternalServerError)
            })
        )
    }
}

pub struct PreRegister(pub AuthRequest);

impl Message for PreRegister {
    type Result = Result<AuthRequest, ServiceError>;
}

impl Handler<PreRegister> for PostgresConnection {
    type Result = ResponseFuture<AuthRequest, ServiceError>;

    fn handle(&mut self, msg: PreRegister, _: &mut Self::Context) -> Self::Result {
        let req = msg.0;
        let query = format!(
            "SELECT username, email FROM users
             WHERE username='{}' OR email='{}'", req.username, req.email.as_ref().unwrap());

        Box::new(self.db
            .as_mut()
            .unwrap()
            .simple_query(&query)
            .into_future()
            .map_err(|(e, _)| e)
            .from_err()
            .and_then(|(r, _)| match r {
                Some(s) => match s {
                    SimpleQueryMessage::Row(row) => unique_username_email_check(&req.username, row),
                    _ => Ok(req)
                }
                None => Err(ServiceError::BadRequest)
            })
        )
    }
}

pub struct Register(pub AuthRequest, pub GlobalGuard);

impl Message for Register {
    type Result = Result<Vec<User>, ServiceError>;
}

impl Handler<Register> for PostgresConnection {
    type Result = ResponseFuture<Vec<User>, ServiceError>;

    fn handle(&mut self, msg: Register, _: &mut Self::Context) -> Self::Result {
        let req = msg.0;

        let hash = match hash::hash_password(&req.password) {
            Ok(hash) => hash,
            Err(e) => return Box::new(future::err(e))
        };
        let id = match msg.1.lock() {
            Ok(mut var) => var.next_uid(),
            Err(_) => return Box::new(future::err(ServiceError::InternalServerError))
        };
        let u = match req.make_user(&id, &hash) {
            Ok(u) => u,
            Err(e) => return Box::new(future::err(e))
        };
        let query = format!(
            "INSERT INTO users (id, username, email, hashed_password, avatar_url, signature)
             VALUES ('{}', '{}', '{}', '{}', '{}', '{}')
             RETURNING *", u.id, u.username, u.email, u.hashed_password, u.avatar_url, u.signature);

        Box::new(self.db
            .as_mut()
            .unwrap()
            .simple_query(&query)
            .into_future()
            .map_err(|(e, _)| e)
            .from_err()
            .and_then(move |(r, _)| match r {
                Some(s) => match s {
                    SimpleQueryMessage::Row(row) => user_from_simple_row(row).map(|u| vec![u]),
                    _ => return Err(ServiceError::InternalServerError)
                }
                None => return Err(ServiceError::InternalServerError)
            })
        )
    }
}

pub struct Login(pub AuthRequest);

impl Message for Login {
    type Result = Result<AuthResponse, ServiceError>;
}

impl Handler<Login> for PostgresConnection {
    type Result = ResponseFuture<AuthResponse, ServiceError>;

    fn handle(&mut self, msg: Login, _: &mut Self::Context) -> Self::Result {
        let req = msg.0;
        let query = format!("SELECT * FROM users WHERE username='{}'", &req.username);

        Box::new(self.db
            .as_mut()
            .unwrap()
            .simple_query(&query)
            .into_future()
            .map_err(|(e, _)| e)
            .from_err()
            .and_then(move |(r, _)| match r {
                Some(s) => match s {
                    SimpleQueryMessage::Row(row) => auth_response_from_simple_row(row, &req.password),
                    _ => return Err(ServiceError::InvalidUsername)
                }
                None => return Err(ServiceError::InternalServerError)
            })
        )
    }
}

// helper functions
fn unique_username_email_check(
    username: &str,
    row: SimpleQueryRow,
) -> Result<AuthRequest, ServiceError> {
    let row0 = row.get(0).ok_or(ServiceError::InternalServerError)?;
    if row0 == username {
        Err(ServiceError::UsernameTaken)
    } else {
        Err(ServiceError::EmailTaken)
    }
}

fn auth_response_from_simple_row(
    row: SimpleQueryRow,
    pass: &str,
) -> Result<AuthResponse, ServiceError> {
    let hash = row.get(3).ok_or(ServiceError::InternalServerError)?;
    let _ = hash::verify_password(pass, hash)?;

    let user = user_from_simple_row(row)?;
    let token = jwt::JwtPayLoad::new(user.id, user.is_admin).sign()?;

    Ok(AuthResponse { token, user })
}

fn user_from_simple_row(
    row: SimpleQueryRow
) -> Result<User, ServiceError> {
    Ok(User {
        id: row.get(0).ok_or(ServiceError::InternalServerError)?.parse::<u32>().map_err(|_| ServiceError::InternalServerError)?,
        username: row.get(1).ok_or(ServiceError::InternalServerError)?.to_owned(),
        email: row.get(2).ok_or(ServiceError::InternalServerError)?.to_owned(),
        hashed_password: row.get(3).ok_or(ServiceError::InternalServerError)?.to_owned(),
        avatar_url: row.get(4).ok_or(ServiceError::InternalServerError)?.to_owned(),
        signature: row.get(5).ok_or(ServiceError::InternalServerError)?.to_owned(),
        created_at: NaiveDateTime::parse_from_str(row.get(6).ok_or(ServiceError::InternalServerError)?, "%Y-%m-%d %H:%M:%S%.f")?,
        updated_at: NaiveDateTime::parse_from_str(row.get(7).ok_or(ServiceError::InternalServerError)?, "%Y-%m-%d %H:%M:%S%.f")?,
        is_admin: row.get(8).ok_or(ServiceError::InternalServerError)?.parse::<u32>().map_err(|_| ServiceError::InternalServerError)?,
        blocked: if row.get(9) == Some("f") { false } else { true },
        show_email: if row.get(10) == Some("f") { false } else { true },
        show_created_at: if row.get(11) == Some("f") { false } else { true },
        show_updated_at: if row.get(12) == Some("f") { false } else { true },
    })
}

fn topic_from_simple_row(
    row: SimpleQueryRow
) -> Result<Vec<Topic>, ServiceError> {
    let mut vec = Vec::with_capacity(1);
    vec.push(Topic {
        id: row.get(0).ok_or(ServiceError::InternalServerError)?.parse::<u32>().map_err(|_| ServiceError::InternalServerError)?,
        user_id: row.get(1).ok_or(ServiceError::InternalServerError)?.parse::<u32>().map_err(|_| ServiceError::InternalServerError)?,
        category_id: row.get(2).ok_or(ServiceError::InternalServerError)?.parse::<u32>().map_err(|_| ServiceError::InternalServerError)?,
        title: row.get(3).ok_or(ServiceError::InternalServerError)?.to_owned(),
        body: row.get(4).ok_or(ServiceError::InternalServerError)?.to_owned(),
        thumbnail: row.get(5).ok_or(ServiceError::InternalServerError)?.to_owned(),
        created_at: NaiveDateTime::parse_from_str(row.get(6).ok_or(ServiceError::InternalServerError)?, "%Y-%m-%d %H:%M:%S%.f")?,
        updated_at: NaiveDateTime::parse_from_str(row.get(7).ok_or(ServiceError::InternalServerError)?, "%Y-%m-%d %H:%M:%S%.f")?,
        last_reply_time: NaiveDateTime::parse_from_str(row.get(8).ok_or(ServiceError::InternalServerError)?, "%Y-%m-%d %H:%M:%S%.f")?,
        reply_count: row.get(9).ok_or(ServiceError::InternalServerError)?.parse::<i32>().map_err(|_| ServiceError::InternalServerError)?,
        is_locked: if row.get(10) == Some("f") { false } else { true },
    });
    Ok(vec)
}

fn topic_from_row(
    row: Option<Row>
) -> Result<Vec<Topic>, ServiceError> {
    match row {
        Some(row) => {
            let mut vec = Vec::with_capacity(1);
            vec.push(Topic {
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
            Ok(vec)
        }
        None => Err(ServiceError::InternalServerError)
    }
}

