use std::convert::TryFrom;
use futures::Future;

use actix::prelude::*;
use chrono::NaiveDateTime;
use tokio_postgres::{Row, SimpleQueryRow, SimpleQueryMessage, Statement, Client, types::ToSql};

use crate::util::{hash, jwt};

use crate::model::{
    actors::{ErrorReportRecipient, DatabaseService, TalkService},
    common::{GetSelfId, GetUserId},
    errors::{ResError, RepError},
    post::Post,
    user::{User, AuthRequest, AuthResponse},
    category::Category,
    topic::Topic,
    talk::{Talk, PublicMessage, PrivateMessage},
};

impl DatabaseService {
    pub fn simple_query_single_row<T>(&mut self, query: &str, index: usize) -> impl Future<Item=T, Error=ResError>
        where T: std::str::FromStr + 'static {
        self.simple_query_single_row_trait(query, index)
    }

    pub fn simple_query_one<T>(&mut self, query: &str) -> impl Future<Item=T, Error=ResError>
        where T: TryFrom<SimpleQueryRow, Error=ResError> + 'static {
        self.simple_query_one_trait(query)
    }

    pub fn simple_query_multi<T>(&mut self, query: &str, vec: Vec<T>) -> Box<dyn Future<Item=Vec<T>, Error=ResError>>
        where T: TryFrom<SimpleQueryRow, Error=ResError> + 'static {
        self.simple_query_multi_trait(query, vec)
    }

    pub fn insert_topic(&mut self, p: &[&dyn ToSql]) -> impl Future<Item=Topic, Error=ResError> {
        Self::query_one_trait(
            self.db.as_mut().unwrap(),
            self.insert_topic.as_ref().unwrap(),
            p,
            self.error_reprot.as_ref().map(Clone::clone))
    }

    pub fn insert_post(&mut self, p: &[&dyn ToSql]) -> impl Future<Item=Post, Error=ResError> {
        Self::query_one_trait(
            self.db.as_mut().unwrap(),
            self.insert_post.as_ref().unwrap(),
            p,
            self.error_reprot.as_ref().map(Clone::clone))
    }

    pub fn insert_user(&mut self, p: &[&dyn ToSql]) -> impl Future<Item=User, Error=ResError> {
        Self::query_one_trait(
            self.db.as_mut().unwrap(),
            self.insert_user.as_ref().unwrap(),
            p,
            self.error_reprot.as_ref().map(Clone::clone))
    }

    pub fn get_users_by_id(&mut self, ids: &Vec<u32>) -> impl Future<Item=Vec<User>, Error=ResError> {
        Self::query_multi(
            self.db.as_mut().unwrap(),
            self.users_by_id.as_ref().unwrap(),
            &[ids],
            Vec::with_capacity(ids.len()),
            self.error_reprot.as_ref().map(Clone::clone))
    }

    pub fn get_topics_by_id_with_uid(&mut self, ids: Vec<u32>) -> impl Future<Item=(Vec<Topic>, Vec<u32>), Error=ResError> {
        Self::query_multi_with_uid(
            self.db.as_mut().unwrap(),
            self.topics_by_id.as_ref().unwrap(),
            ids,
            self.error_reprot.as_ref().map(Clone::clone))
    }

    pub fn get_posts_by_id_with_uid(&mut self, ids: Vec<u32>) -> impl Future<Item=(Vec<Post>, Vec<u32>), Error=ResError> {
        Self::query_multi_with_uid(
            self.db.as_mut().unwrap(),
            self.posts_by_id.as_ref().unwrap(),
            ids,
            self.error_reprot.as_ref().map(Clone::clone))
    }

    pub fn unique_username_email_check(&mut self, q: &str, req: AuthRequest) -> impl Future<Item=AuthRequest, Error=ResError> {
        self.simple_query_row_trait(q)
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
            })
    }

    pub fn generate_auth_response(&mut self, q: &str, pass: String) -> impl Future<Item=AuthResponse, Error=ResError> {
        self.simple_query_row_trait(q)
            .and_then(move |r| {
                let hash = r.get(3).ok_or(ResError::InternalServerError)?;
                let _ = hash::verify_password(pass.as_str(), hash)?;

                let user = User::try_from(r)?;
                let token = jwt::JwtPayLoad::new(user.id, user.privilege).sign()?;

                Ok(AuthResponse { token, user })
            }
            )
    }
}

impl TalkService {
    pub fn insert_pub_msg(&mut self, p: &[&dyn ToSql]) -> impl Future<Item=PublicMessage, Error=ResError> {
        Self::query_one_trait(
            self.db.as_mut().unwrap(),
            self.insert_pub_msg.as_ref().unwrap(),
            p,
            None)
    }

    pub fn insert_prv_msg(&mut self, p: &[&dyn ToSql]) -> impl Future<Item=PrivateMessage, Error=ResError> {
        Self::query_one_trait(
            self.db.as_mut().unwrap(),
            self.insert_prv_msg.as_ref().unwrap(),
            p,
            None)
    }

    pub fn get_pub_msg(&mut self, p: &[&dyn ToSql]) -> impl Future<Item=Vec<PublicMessage>, Error=ResError> {
        Self::query_multi(
            self.db.as_mut().unwrap(),
            self.get_pub_msg.as_ref().unwrap(),
            p,
            Vec::with_capacity(20),
            None)
    }

    pub fn join_talk(&mut self, p: &[&dyn ToSql]) -> impl Future<Item=Talk, Error=ResError> {
        Self::query_one_trait(
            self.db.as_mut().unwrap(),
            self.join_talk.as_ref().unwrap(),
            p,
            None)
    }

    pub fn simple_query_one<T>(&mut self, query: &str) -> impl Future<Item=T, Error=ResError>
        where T: TryFrom<SimpleQueryRow, Error=ResError> + 'static {
        self.simple_query_one_trait(query)
    }

    pub fn simple_query_single_row<T>(&mut self, query: &str, index: usize) -> impl Future<Item=T, Error=ResError>
        where T: std::str::FromStr + 'static {
        self.simple_query_single_row_trait(query, index)
    }

    pub fn simple_query_row(&mut self, query: &str) -> impl Future<Item=SimpleQueryRow, Error=ResError> {
        self.simple_query_row_trait(query)
    }
}


trait SimpleQuery {
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


trait Query {
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


trait SimpleQueryOne
    where Self: SimpleQuery {
    fn simple_query_single_row_trait<T>(&mut self, q: &str, i: usize) -> Box<dyn Future<Item=T, Error=ResError>>
        where T: std::str::FromStr + 'static {
        Box::new(self
            .simple_query_row_trait(q)
            .and_then(move |r| r
                .get(i)
                .ok_or(ResError::BadRequest)?
                .parse::<T>()
                .map_err(|_| ResError::ParseError)))
    }

    fn simple_query_one_trait<T>(&mut self, q: &str) -> Box<dyn Future<Item=T, Error=ResError>>
        where T: TryFrom<SimpleQueryRow, Error=ResError> + 'static {
        Box::new(self
            .simple_query_row_trait(q)
            .and_then(T::try_from))
    }

    fn simple_query_row_trait(&mut self, q: &str) -> Box<dyn Future<Item=SimpleQueryRow, Error=ResError>> {
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


trait SimpleQueryMulti
    where Self: SimpleQuery {
    fn simple_query_multi_trait<T>(&mut self, q: &str, vec: Vec<T>) -> Box<dyn Future<Item=Vec<T>, Error=ResError>>
        where T: TryFrom<SimpleQueryRow, Error=ResError> + 'static {
        Box::new(self
            .simple_query_stream(q)
            .fold(vec, move |mut vec, r| {
                if let SimpleQueryMessage::Row(r) = r {
                    if let Some(v) = T::try_from(r).ok() {
                        vec.push(v);
                    }
                }
                Ok::<_, ResError>(vec)
            }))
    }
}

impl SimpleQueryMulti for DatabaseService {}


trait QueryOne
    where Self: Query {
    fn query_one_trait<T>(
        c: &mut Client,
        st: &Statement,
        p: &[&dyn ToSql],
        rep: Option<ErrorReportRecipient>,
    ) -> Box<dyn Future<Item=T, Error=ResError>>
        where T: TryFrom<Row, Error=ResError> + 'static {
        Box::new(Self::query_stream(c, st, p, rep)
            .into_future()
            .map_err(|(e, _)| e)
            .and_then(|(r, _)| r.ok_or(ResError::BadRequest))
            .and_then(T::try_from))
    }
}

impl QueryOne for DatabaseService {}

impl QueryOne for TalkService {}


trait QueryMulti
    where Self: Query {
    fn query_multi<T>(
        c: &mut Client,
        st: &Statement,
        p: &[&dyn ToSql],
        vec: Vec<T>,
        rep: Option<ErrorReportRecipient>,
    ) -> Box<dyn Future<Item=Vec<T>, Error=ResError>>
        where T: TryFrom<Row> + 'static {
        Box::new(Self::query_stream(c, st, p, rep)
            .fold(vec, move |mut vec, r| {
                if let Some(r) = T::try_from(r).ok() {
                    vec.push(r);
                }
                Ok::<_, ResError>(vec)
            }))
    }
}

impl QueryMulti for DatabaseService {}

impl QueryMulti for TalkService {}


trait QueryMultiWithUids
    where Self: Query {
    fn query_multi_with_uid<T>(
        c: &mut Client,
        st: &Statement,
        ids_org: Vec<u32>,
        rep: Option<ErrorReportRecipient>,
    ) -> Box<dyn Future<Item=(Vec<T>, Vec<u32>), Error=ResError>>
        where T: TryFrom<Row> + GetSelfId + GetUserId + 'static {
        let len = ids_org.len();
        let vec = Vec::with_capacity(len);
        let ids = Vec::with_capacity(len);

        Box::new(Self::query_stream(c, st, &[&ids_org], rep)
            .fold((vec, ids), move |(mut vec, mut ids), r| {
                if let Some(v) = T::try_from(r).ok() {
                    ids.push(v.get_user_id());
                    vec.push(v);
                }
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


impl TryFrom<Row> for User {
    type Error = ResError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(User {
            id: row.try_get(0)?,
            username: row.try_get(1)?,
            email: row.try_get(2)?,
            hashed_password: "1".to_owned(),
            avatar_url: row.try_get(4)?,
            signature: row.try_get(5)?,
            created_at: row.try_get(6)?,
            privilege: row.try_get(7)?,
            show_email: row.try_get(8)?,
            online_status: None,
            last_online: None,
        })
    }
}

impl TryFrom<Row> for Topic {
    type Error = ResError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Topic {
            id: row.try_get(0)?,
            user_id: row.try_get(1)?,
            category_id: row.try_get(2)?,
            title: row.try_get(3)?,
            body: row.try_get(4)?,
            thumbnail: row.try_get(5)?,
            created_at: row.try_get(6)?,
            updated_at: row.try_get(7)?,
            last_reply_time: None,
            is_locked: row.try_get(8)?,
            reply_count: None,
        })
    }
}

impl TryFrom<Row> for Post {
    type Error = ResError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Post {
            id: row.try_get(0)?,
            user_id: row.try_get(1)?,
            topic_id: row.try_get(2)?,
            category_id: row.try_get(3)?,
            post_id: row.try_get(4)?,
            post_content: row.try_get(5)?,
            created_at: row.try_get(6)?,
            updated_at: row.try_get(7)?,
            last_reply_time: None,
            is_locked: row.try_get(8)?,
            reply_count: None,
        })
    }
}

impl TryFrom<Row> for Talk {
    type Error = ResError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Talk {
            id: row.try_get(0)?,
            name: row.try_get(1)?,
            description: row.try_get(2)?,
            secret: row.try_get(3)?,
            privacy: row.try_get(4)?,
            owner: row.try_get(5)?,
            admin: row.try_get(6)?,
            users: row.try_get(7)?,
        })
    }
}

impl TryFrom<Row> for PublicMessage {
    type Error = ResError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(PublicMessage {
            talk_id: row.try_get(0)?,
            time: row.try_get(1)?,
            text: row.try_get(2)?,
        })
    }
}

impl TryFrom<Row> for PrivateMessage {
    type Error = ResError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(PrivateMessage {
            user_id: row.try_get(0)?,
            time: row.try_get(2)?,
            text: row.try_get(3)?,
        })
    }
}

impl TryFrom<SimpleQueryRow> for Post {
    type Error = ResError;
    fn try_from(r: SimpleQueryRow) -> Result<Self, Self::Error> {
        let post_id = match r.get(4) {
            Some(s) => s.parse::<u32>().ok(),
            None => None
        };
        Ok(Post {
            id: r.get(0).ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            user_id: r.get(1).ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            topic_id: r.get(2).ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            category_id: r.get(3).ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            post_id,
            post_content: r.get(5).ok_or(ResError::DataBaseReadError)?.to_owned(),
            created_at: NaiveDateTime::parse_from_str(r.get(6).ok_or(ResError::DataBaseReadError)?, "%Y-%m-%d %H:%M:%S%.f")?,
            updated_at: NaiveDateTime::parse_from_str(r.get(7).ok_or(ResError::DataBaseReadError)?, "%Y-%m-%d %H:%M:%S%.f")?,
            last_reply_time: None,
            is_locked: if r.get(8) == Some("f") { false } else { true },
            reply_count: None,
        })
    }
}

impl TryFrom<SimpleQueryRow> for Topic {
    type Error = ResError;
    fn try_from(r: SimpleQueryRow) -> Result<Self, Self::Error> {
        Ok(Topic {
            id: r.get(0).ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            user_id: r.get(1).ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            category_id: r.get(2).ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            title: r.get(3).ok_or(ResError::DataBaseReadError)?.to_owned(),
            body: r.get(4).ok_or(ResError::DataBaseReadError)?.to_owned(),
            thumbnail: r.get(5).ok_or(ResError::DataBaseReadError)?.to_owned(),
            created_at: NaiveDateTime::parse_from_str(r.get(6).ok_or(ResError::DataBaseReadError)?, "%Y-%m-%d %H:%M:%S%.f")?,
            updated_at: NaiveDateTime::parse_from_str(r.get(7).ok_or(ResError::DataBaseReadError)?, "%Y-%m-%d %H:%M:%S%.f")?,
            last_reply_time: None,
            is_locked: if r.get(8) == Some("f") { false } else { true },
            reply_count: None,
        })
    }
}

impl TryFrom<SimpleQueryRow> for User {
    type Error = ResError;
    fn try_from(r: SimpleQueryRow) -> Result<Self, Self::Error> {
        Ok(User {
            id: r.get(0).ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            username: r.get(1).ok_or(ResError::DataBaseReadError)?.to_owned(),
            email: r.get(2).ok_or(ResError::DataBaseReadError)?.to_owned(),
            hashed_password: r.get(3).ok_or(ResError::DataBaseReadError)?.to_owned(),
            avatar_url: r.get(4).ok_or(ResError::DataBaseReadError)?.to_owned(),
            signature: r.get(5).ok_or(ResError::DataBaseReadError)?.to_owned(),
            created_at: NaiveDateTime::parse_from_str(r.get(6).ok_or(ResError::DataBaseReadError)?, "%Y-%m-%d %H:%M:%S%.f")?,
            privilege: r.get(7).ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            show_email: if r.get(8) == Some("f") { false } else { true },
            online_status: None,
            last_online: None,
        })
    }
}

impl TryFrom<SimpleQueryRow> for Category {
    type Error = ResError;
    fn try_from(r: SimpleQueryRow) -> Result<Self, Self::Error> {
        Ok(Category {
            id: r.get(0).ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            name: r.get(1).ok_or(ResError::DataBaseReadError)?.to_owned(),
            thumbnail: r.get(2).ok_or(ResError::DataBaseReadError)?.to_owned(),
            topic_count: None,
            post_count: None,
            topic_count_new: None,
            post_count_new: None,
        })
    }
}

impl TryFrom<SimpleQueryRow> for Talk {
    type Error = ResError;
    fn try_from(r: SimpleQueryRow) -> Result<Self, Self::Error> {
        Ok(Talk {
            id: r.get(0).ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            name: r.get(1).ok_or(ResError::DataBaseReadError)?.to_owned(),
            description: r.get(2).ok_or(ResError::DataBaseReadError)?.to_owned(),
            secret: r.get(3).ok_or(ResError::DataBaseReadError)?.to_owned(),
            privacy: r.get(4).ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            owner: r.get(5).ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            admin: vec![],
            users: vec![],
        })
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
    where T: TryFrom<SimpleQueryRow> {
    c.simple_query(&q)
        .from_err()
        .fold(Vec::new(), move |mut vec, row| {
            if let SimpleQueryMessage::Row(row) = row {
                if let Some(v) = T::try_from(row).ok() {
                    vec.push(v)
                }
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