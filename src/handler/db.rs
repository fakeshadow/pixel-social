use std::{
    cell::{RefCell, RefMut},
    convert::TryFrom,
};

use futures::{future::join_all, Future, Stream};
use tokio_postgres::{
    connect, types::ToSql, Client, NoTls, Row, SimpleQueryMessage, SimpleQueryRow, Statement,
};

use crate::model::actors::PSNService;
use crate::model::{
    actors::TalkService,
    common::{SelfId, SelfUserId},
    errors::ResError,
    user::AuthRequest,
};

// database service is not an actor.
pub struct DatabaseService {
    pub url: String,
    pub db: RefCell<Client>,
    pub topics_by_id: RefCell<Statement>,
    pub posts_by_id: RefCell<Statement>,
    pub users_by_id: RefCell<Statement>,
    pub insert_topic: RefCell<Statement>,
    pub insert_post: RefCell<Statement>,
    pub insert_user: RefCell<Statement>,
}

impl DatabaseService {
    pub fn check_conn(
        &self,
    ) -> impl Future<Item = Option<(Client, Vec<Statement>)>, Error = ResError> {
        use futures::future::Either;
        if self.db.borrow().is_closed() {
            Either::A(DatabaseService::connect(self.url.as_str()))
        } else {
            Either::B(futures::future::ok(None))
        }
    }

    pub fn init(postgres_url: &str) -> impl Future<Item = DatabaseService, Error = ResError> {
        let url = postgres_url.to_owned();
        DatabaseService::connect(url.as_str()).and_then(|opt| {
            let (c, mut v) = opt.unwrap();
            let topics_by_id = v.pop().unwrap();
            let posts_by_id = v.pop().unwrap();
            let users_by_id = v.pop().unwrap();
            let insert_topic = v.pop().unwrap();
            let insert_post = v.pop().unwrap();
            let insert_user = v.pop().unwrap();

            Ok(DatabaseService {
                url,
                db: RefCell::new(c),
                topics_by_id: RefCell::new(topics_by_id),
                posts_by_id: RefCell::new(posts_by_id),
                users_by_id: RefCell::new(users_by_id),
                insert_topic: RefCell::new(insert_topic),
                insert_post: RefCell::new(insert_post),
                insert_user: RefCell::new(insert_user),
            })
        })
    }

    pub fn if_replace_db(&self, opt: Option<(Client, Vec<Statement>)>) -> &Self {
        if let Some((c, mut v)) = opt {
            let topics_by_id = v.pop().unwrap();
            let posts_by_id = v.pop().unwrap();
            let users_by_id = v.pop().unwrap();
            let insert_topic = v.pop().unwrap();
            let insert_post = v.pop().unwrap();
            let insert_user = v.pop().unwrap();

            self.db.replace(c);
            self.topics_by_id.replace(topics_by_id);
            self.posts_by_id.replace(posts_by_id);
            self.users_by_id.replace(users_by_id);
            self.insert_topic.replace(insert_topic);
            self.insert_post.replace(insert_post);
            self.insert_user.replace(insert_user);
        }
        self
    }

    pub fn connect(
        postgres_url: &str,
    ) -> impl Future<Item = Option<(Client, Vec<Statement>)>, Error = ResError> {
        connect(postgres_url, NoTls)
            .from_err()
            .and_then(|(mut c, conn)| {
                actix::spawn(conn.map_err(|_| ()));

                let p1 = c.prepare("SELECT * FROM topics WHERE id=ANY($1)");
                let p2 = c.prepare("SELECT * FROM posts WHERE id=ANY($1)");
                let p3 = c.prepare("SELECT * FROM users WHERE id=ANY($1)");
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
                    .from_err()
                    .map(|v| Some((c, v)))
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

    /// when folding stream into data struct the error from parsing column are ignored.
    /// We send all the good data to frontend.
    /// this also applies to simple_query_multi_trait.
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
            if let Ok(r) = T::try_from(r) {
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

impl Query for PSNService {
    fn get_client(&self) -> RefMut<Client> {
        self.db.as_ref().unwrap().borrow_mut()
    }
}

pub trait SimpleQuery {
    fn simple_query_single_row_trait<T>(
        &self,
        query: &str,
        column_index: usize,
    ) -> Box<dyn Future<Item = T, Error = ResError>>
    where
        T: std::str::FromStr + 'static,
    {
        Box::new(self.simple_query_row_trait(query).and_then(move |r| {
            r.get(column_index)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<T>()
                .map_err(|_| ResError::ParseError)
        }))
    }

    fn simple_query_one_trait<T>(&self, query: &str) -> Box<dyn Future<Item = T, Error = ResError>>
    where
        T: TryFrom<SimpleQueryRow, Error = ResError> + 'static,
    {
        Box::new(self.simple_query_row_trait(query).and_then(T::try_from))
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
                if let Ok(v) = T::try_from(r) {
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

impl SimpleQuery for PSNService {
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
        T: SelfUserId + SelfId + TryFrom<Row, Error = ResError> + 'static,
    {
        self.query_trait(st, &[&ids])
            .fold(
                (Vec::with_capacity(20), Vec::with_capacity(20)),
                move |(mut v, mut ids), r| {
                    if let Ok(r) = T::try_from(r) {
                        ids.push(r.get_user_id());
                        v.push(r)
                    }
                    Ok::<_, ResError>((v, ids))
                },
            )
            .map(move |(mut v, uids)| {
                let mut result = Vec::with_capacity(v.len());
                for id in ids.iter() {
                    for (i, idv) in v.iter().enumerate() {
                        if id == &idv.self_id() {
                            result.push(v.swap_remove(i));
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
        ids: &[u32],
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
            if let Ok(r) = r {
                if let Some(r) = r.get(0) {
                    if r == req.username.as_str() {
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

// helper functions for build cache on startup
/// this function will cause a postgres error SqlState("42P01") as we try to load categories table beforehand to prevent unwanted table creation.
/// it's safe to ignore this error when create db tables.
pub fn load_all<T>(c: &mut Client, q: &str) -> impl Future<Item = Vec<T>, Error = ResError>
where
    T: TryFrom<SimpleQueryRow>,
{
    c.simple_query(&q)
        .from_err()
        .fold(Vec::new(), move |mut vec, row| {
            if let SimpleQueryMessage::Row(row) = row {
                if let Ok(v) = T::try_from(row) {
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
