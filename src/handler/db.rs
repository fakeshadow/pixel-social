use futures::{future::join_all, Future, Stream};
use std::{cell::RefMut, convert::TryFrom};

use chrono::NaiveDateTime;
use tokio_postgres::{
    connect, types::ToSql, Client, NoTls, Row, SimpleQueryMessage, SimpleQueryRow, Statement,
};

use crate::model::{
    actors::TalkService,
    category::Category,
    common::{GetSelfId, GetUserId},
    errors::ResError,
    post::Post,
    talk::{PrivateMessage, PublicMessage, Relation, Talk},
    topic::Topic,
    user::{AuthRequest, User},
};

// database service is not an actor.
pub struct DatabaseService {
    pub db: std::cell::RefCell<Client>,
    pub topics_by_id: Statement,
    pub posts_by_id: Statement,
    pub users_by_id: Statement,
    pub insert_topic: Statement,
    pub insert_post: Statement,
    pub insert_user: Statement,
}

impl DatabaseService {
    pub fn init(postgres_url: &str) -> impl Future<Item = DatabaseService, Error = ()> {
        connect(postgres_url, NoTls)
            .map_err(|e| panic!("{:?}", e))
            .and_then(|(mut c, conn)| {
                actix_rt::spawn(conn.map_err(|e| panic!("{}", e)));

                let p1 = c.prepare("SELECT * FROM topics WHERE id = ANY($1)");
                let p2 = c.prepare("SELECT * FROM posts WHERE id = ANY($1)");
                let p3 = c.prepare("SELECT * FROM users WHERE id = ANY($1)");
                let p4 = c.prepare("INSERT INTO topics
                       (id, user_id, category_id, thumbnail, title, body, created_at, updated_at)
                       VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                       RETURNING *");
                let p5 = c.prepare("INSERT INTO posts
                       (id, user_id, topic_id, category_id, post_id, post_content, created_at, updated_at)
                       VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                       RETURNING *");
                let p6 = c.prepare("INSERT INTO users
                       (id, username, email, hashed_password, avatar_url, signature)
                       VALUES ($1, $2, $3, $4, $5, $6)
                       RETURNING *");

                join_all(vec![p6, p5, p4, p3, p2, p1])
                    .map_err(move |e| panic!("{:?}", e))
                    .map(|mut v: Vec<Statement>| {
                        let topics_by_id = v.pop().unwrap();
                        let posts_by_id = v.pop().unwrap();
                        let users_by_id = v.pop().unwrap();
                        let insert_topic = v.pop().unwrap();
                        let insert_post = v.pop().unwrap();
                        let insert_user = v.pop().unwrap();

                        DatabaseService {
                            db: std::cell::RefCell::new(c),
                            topics_by_id,
                            posts_by_id,
                            users_by_id,
                            insert_topic,
                            insert_post,
                            insert_user,
                        }
                    })
            })
    }
}

pub trait Query {
    fn query_trait(
        &self,
        st: &Statement,
        p: &[&dyn ToSql],
    ) -> Box<dyn Stream<Item = Row, Error = ResError>> {
        Box::new(self.get_client().query(st, p).from_err())
    }

    fn query_one_trait<T>(
        &self,
        st: &Statement,
        p: &[&dyn ToSql],
    ) -> Box<dyn Future<Item = T, Error = ResError>>
    where
        T: TryFrom<Row, Error = ResError> + 'static,
    {
        Box::new(
            self.query_trait(st, p)
                .into_future()
                .map_err(|(e, _)| e)
                .and_then(|(r, _)| r.ok_or(ResError::BadRequest))
                .and_then(T::try_from),
        )
    }

    fn query_multi_trait<T>(
        &self,
        st: &Statement,
        p: &[&dyn ToSql],
        vec: Vec<T>,
    ) -> Box<dyn Future<Item = Vec<T>, Error = ResError>>
    where
        T: TryFrom<Row, Error = ResError> + 'static,
    {
        Box::new(self.query_trait(st, p).fold(vec, move |mut vec, r| {
            if let Some(r) = T::try_from(r).ok() {
                vec.push(r);
            }
            Ok::<_, ResError>(vec)
        }))
    }

    fn get_client(&self) -> RefMut<Client>;
}

impl Query for DatabaseService {
    fn get_client(&self) -> RefMut<Client> {
        self.db.borrow_mut()
    }
}

impl Query for TalkService {
    fn get_client(&self) -> RefMut<Client> {
        self.db.borrow_mut()
    }
}

pub trait SimpleQuery {
    fn simple_query_single_row_trait<T>(
        &self,
        q: &str,
        i: usize,
    ) -> Box<dyn Future<Item = T, Error = ResError>>
    where
        T: std::str::FromStr + 'static,
    {
        Box::new(self.simple_query_row_trait(q).and_then(move |r| {
            r.get(i)
                .ok_or(ResError::BadRequest)?
                .parse::<T>()
                .map_err(|_| ResError::ParseError)
        }))
    }

    fn simple_query_one_trait<T>(&self, q: &str) -> Box<dyn Future<Item = T, Error = ResError>>
    where
        T: TryFrom<SimpleQueryRow, Error = ResError> + 'static,
    {
        Box::new(self.simple_query_row_trait(q).and_then(T::try_from))
    }

    fn simple_query_multi_trait<T>(
        &self,
        q: &str,
        vec: Vec<T>,
    ) -> Box<dyn Future<Item = Vec<T>, Error = ResError>>
    where
        T: TryFrom<SimpleQueryRow, Error = ResError> + 'static,
    {
        Box::new(self.simple_query_trait(q).fold(vec, move |mut vec, r| {
            if let SimpleQueryMessage::Row(r) = r {
                if let Some(v) = T::try_from(r).ok() {
                    vec.push(v);
                }
            }
            Ok::<_, ResError>(vec)
        }))
    }

    fn simple_query_row_trait(
        &self,
        q: &str,
    ) -> Box<dyn Future<Item = SimpleQueryRow, Error = ResError>> {
        Box::new(
            self.simple_query_trait(q)
                .into_future()
                .map_err(|(e, _)| e)
                .and_then(|(r, _)| match r {
                    Some(m) => match m {
                        SimpleQueryMessage::Row(r) => Ok(r),
                        _ => Err(ResError::NoContent),
                    },
                    None => Err(ResError::BadRequest),
                }),
        )
    }

    fn simple_query_trait(
        &self,
        query: &str,
    ) -> Box<dyn Stream<Item = SimpleQueryMessage, Error = ResError>> {
        Box::new(self.get_client_simple().simple_query(query).from_err())
    }

    fn get_client_simple(&self) -> RefMut<Client>;
}

impl SimpleQuery for DatabaseService {
    fn get_client_simple(&self) -> RefMut<Client> {
        self.get_client()
    }
}

impl SimpleQuery for TalkService {
    fn get_client_simple(&self) -> RefMut<Client> {
        self.get_client()
    }
}

impl DatabaseService {
    pub fn get_by_id_with_uid<T>(
        &self,
        st: &Statement,
        ids: Vec<u32>,
    ) -> impl Future<Item = (Vec<T>, Vec<u32>), Error = ResError>
    where
        T: GetUserId + GetSelfId + TryFrom<Row, Error = ResError> + 'static,
    {
        self.query_trait(st, &[&ids])
            .fold(
                (Vec::with_capacity(20), Vec::with_capacity(20)),
                move |(mut v, mut ids), r| {
                    if let Some(r) = T::try_from(r).ok() {
                        ids.push(r.get_user_id());
                        v.push(r)
                    }
                    Ok::<_, ResError>((v, ids))
                },
            )
            .map(move |(mut v, uids)| {
                let mut result = Vec::with_capacity(v.len());
                for i in 0..ids.len() {
                    for j in 0..v.len() {
                        if &ids[i] == v[j].self_id() {
                            result.push(v.swap_remove(j));
                            break;
                        }
                    }
                }
                (result, uids)
            })
    }

    pub fn get_by_id<T>(
        &self,
        st: &Statement,
        ids: &Vec<u32>,
    ) -> impl Future<Item = Vec<T>, Error = ResError>
    where
        T: TryFrom<Row, Error = ResError> + 'static,
    {
        self.query_multi_trait(st, &[&ids], Vec::with_capacity(21))
    }

    pub fn unique_username_email_check(
        &self,
        q: &str,
        req: AuthRequest,
    ) -> impl Future<Item = AuthRequest, Error = ResError> {
        self.simple_query_row_trait(q).then(|r| {
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
}

impl TalkService {
    pub fn get_by_time<T>(
        &self,
        st: &Statement,
        p: &[&dyn ToSql],
    ) -> impl Future<Item = Vec<T>, Error = ResError>
    where
        T: TryFrom<Row, Error = ResError> + 'static,
    {
        self.query_multi_trait(st, p, Vec::with_capacity(20))
    }
}

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
            is_locked: row.try_get(8)?,
            is_visible: row.try_get(9)?,
            last_reply_time: None,
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

impl TryFrom<Row> for Relation {
    type Error = ResError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Relation {
            friends: row.try_get(0)?,
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
            None => None,
        };
        Ok(Post {
            id: r
                .get(0)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            user_id: r
                .get(1)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            topic_id: r
                .get(2)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            category_id: r
                .get(3)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            post_id,
            post_content: r.get(5).ok_or(ResError::DataBaseReadError)?.to_owned(),
            created_at: NaiveDateTime::parse_from_str(
                r.get(6).ok_or(ResError::DataBaseReadError)?,
                "%Y-%m-%d %H:%M:%S%.f",
            )?,
            updated_at: NaiveDateTime::parse_from_str(
                r.get(7).ok_or(ResError::DataBaseReadError)?,
                "%Y-%m-%d %H:%M:%S%.f",
            )?,
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
            id: r
                .get(0)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            user_id: r
                .get(1)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            category_id: r
                .get(2)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            title: r.get(3).ok_or(ResError::DataBaseReadError)?.to_owned(),
            body: r.get(4).ok_or(ResError::DataBaseReadError)?.to_owned(),
            thumbnail: r.get(5).ok_or(ResError::DataBaseReadError)?.to_owned(),
            created_at: NaiveDateTime::parse_from_str(
                r.get(6).ok_or(ResError::DataBaseReadError)?,
                "%Y-%m-%d %H:%M:%S%.f",
            )?,
            updated_at: NaiveDateTime::parse_from_str(
                r.get(7).ok_or(ResError::DataBaseReadError)?,
                "%Y-%m-%d %H:%M:%S%.f",
            )?,
            is_locked: if r.get(8) == Some("f") { false } else { true },
            is_visible: if r.get(9) == Some("f") { false } else { true },
            last_reply_time: None,
            reply_count: None,
        })
    }
}

impl TryFrom<SimpleQueryRow> for User {
    type Error = ResError;
    fn try_from(r: SimpleQueryRow) -> Result<Self, Self::Error> {
        Ok(User {
            id: r
                .get(0)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            username: r.get(1).ok_or(ResError::DataBaseReadError)?.to_owned(),
            email: r.get(2).ok_or(ResError::DataBaseReadError)?.to_owned(),
            hashed_password: r.get(3).ok_or(ResError::DataBaseReadError)?.to_owned(),
            avatar_url: r.get(4).ok_or(ResError::DataBaseReadError)?.to_owned(),
            signature: r.get(5).ok_or(ResError::DataBaseReadError)?.to_owned(),
            created_at: NaiveDateTime::parse_from_str(
                r.get(6).ok_or(ResError::DataBaseReadError)?,
                "%Y-%m-%d %H:%M:%S%.f",
            )?,
            privilege: r
                .get(7)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
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
            id: r
                .get(0)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
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
        let admin = r.get(6).ok_or(ResError::DataBaseReadError)?;
        let users = r.get(7).ok_or(ResError::DataBaseReadError)?;

        let alen = admin.len();
        let ulen = users.len();

        let admin: Vec<&str> = if alen < 2 {
            Vec::with_capacity(0)
        } else {
            admin[1..(alen - 1)].split(",").collect()
        };
        let users: Vec<&str> = if ulen < 2 {
            Vec::with_capacity(0)
        } else {
            users[1..(ulen - 1)].split(",").collect()
        };

        Ok(Talk {
            id: r
                .get(0)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            name: r.get(1).ok_or(ResError::DataBaseReadError)?.to_owned(),
            description: r.get(2).ok_or(ResError::DataBaseReadError)?.to_owned(),
            secret: r.get(3).ok_or(ResError::DataBaseReadError)?.to_owned(),
            privacy: r
                .get(4)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            owner: r
                .get(5)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            admin: admin
                .into_iter()
                .map(|a| a.parse::<u32>())
                .collect::<Result<Vec<u32>, _>>()?,
            users: users
                .into_iter()
                .map(|u| u.parse::<u32>())
                .collect::<Result<Vec<u32>, _>>()?,
        })
    }
}

// helper functions for build cache on startup
pub fn load_all<T>(c: &mut Client, q: &str) -> impl Future<Item = Vec<T>, Error = ResError>
where
    T: TryFrom<SimpleQueryRow>,
{
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
) -> impl Future<Item = T, Error = ResError>
where
    T: std::str::FromStr,
{
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
                _ => Err(ResError::NoContent),
            },
            None => Err(ResError::BadRequest),
        })
}
