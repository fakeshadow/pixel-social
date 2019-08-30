use std::{
    cell::{RefCell, RefMut},
    convert::TryFrom,
    future::Future,
};

use futures::{future::join_all, FutureExt, TryFutureExt, TryStreamExt};
use tokio_postgres::{
    types::{ToSql, Type},
    Client, NoTls, Row, SimpleQueryMessage, SimpleQueryRow, Statement,
};

use crate::model::{
    common::{PinedBoxFutureResult, SelfId, SelfUserId},
    errors::ResError,
};

const SELECT_TOPIC: &str = "SELECT * FROM topics WHERE id=ANY($1)";
const SELECT_POST: &str = "SELECT * FROM posts WHERE id=ANY($1)";
const SELECT_USER: &str = "SELECT * FROM users WHERE id=ANY($1)";

const INSERT_TOPIC: &str = "INSERT INTO topics
(id, user_id, category_id, thumbnail, title, body, created_at, updated_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
RETURNING *";

const INSERT_POST: &str = "INSERT INTO posts
(id, user_id, topic_id, category_id, post_id, post_content, created_at, updated_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
RETURNING *";

const INSERT_USER: &str = "INSERT INTO users
(id, username, email, hashed_password, avatar_url, signature)
VALUES ($1, $2, $3, $4, $5, $6)
RETURNING *";

pub struct DatabaseService {
    pub url: String,
    pub client: RefCell<Client>,
    pub topics_by_id: RefCell<Statement>,
    pub posts_by_id: RefCell<Statement>,
    pub users_by_id: RefCell<Statement>,
    pub insert_topic: RefCell<Statement>,
    pub insert_post: RefCell<Statement>,
    pub insert_user: RefCell<Statement>,
}

impl DatabaseService {
    /// database connection is only checked on insert request.
    /// Connections are not shared between threads so the recovery will happen separately.
    pub(crate) async fn check_conn(&self) -> Result<&Self, ResError> {
        if self.client.borrow().is_closed() {
            let (c, mut sts) = DatabaseService::connect(self.url.as_str()).await?;
            let topics_by_id = sts.pop().unwrap();
            let posts_by_id = sts.pop().unwrap();
            let users_by_id = sts.pop().unwrap();
            let insert_topic = sts.pop().unwrap();
            let insert_post = sts.pop().unwrap();
            let insert_user = sts.pop().unwrap();

            self.client.replace(c);
            self.topics_by_id.replace(topics_by_id);
            self.posts_by_id.replace(posts_by_id);
            self.users_by_id.replace(users_by_id);
            self.insert_topic.replace(insert_topic);
            self.insert_post.replace(insert_post);
            self.insert_user.replace(insert_user);
            Ok(self)
        } else {
            Ok(self)
        }
    }

    pub(crate) async fn init(postgres_url: &str) -> Result<DatabaseService, ResError> {
        let url = postgres_url.to_owned();

        let (c, mut sts) = DatabaseService::connect(postgres_url).await?;

        let topics_by_id = sts.pop().unwrap();
        let posts_by_id = sts.pop().unwrap();
        let users_by_id = sts.pop().unwrap();
        let insert_topic = sts.pop().unwrap();
        let insert_post = sts.pop().unwrap();
        let insert_user = sts.pop().unwrap();

        Ok(DatabaseService {
            url,
            client: RefCell::new(c),
            topics_by_id: RefCell::new(topics_by_id),
            posts_by_id: RefCell::new(posts_by_id),
            users_by_id: RefCell::new(users_by_id),
            insert_topic: RefCell::new(insert_topic),
            insert_post: RefCell::new(insert_post),
            insert_user: RefCell::new(insert_user),
        })
    }

    async fn connect(postgres_url: &str) -> Result<(Client, Vec<Statement>), ResError> {
        let (mut c, conn) = tokio_postgres::connect(postgres_url, NoTls).await?;

        //ToDo: remove compat layer when actix convert to use std::future;
        let conn = conn.map(|_| ());
        actix::spawn(conn.unit_error().boxed().compat());

        let p1 = c.prepare_typed(SELECT_TOPIC, &[Type::OID_ARRAY]);
        let p2 = c.prepare_typed(SELECT_POST, &[Type::OID_ARRAY]);
        let p3 = c.prepare_typed(SELECT_USER, &[Type::OID_ARRAY]);
        let p4 = c.prepare_typed(
            INSERT_TOPIC,
            &[
                Type::OID,
                Type::OID,
                Type::OID,
                Type::VARCHAR,
                Type::VARCHAR,
                Type::VARCHAR,
                Type::TIMESTAMP,
                Type::TIMESTAMP,
            ],
        );
        let p5 = c.prepare_typed(
            INSERT_POST,
            &[
                Type::OID,
                Type::OID,
                Type::OID,
                Type::OID,
                Type::OID,
                Type::VARCHAR,
                Type::TIMESTAMP,
                Type::TIMESTAMP,
            ],
        );
        let p6 = c.prepare_typed(
            INSERT_USER,
            &[
                Type::OID,
                Type::VARCHAR,
                Type::VARCHAR,
                Type::VARCHAR,
                Type::VARCHAR,
                Type::VARCHAR,
            ],
        );

        let vec: Vec<Result<Statement, tokio_postgres::Error>> =
            join_all(vec![p6, p5, p4, p3, p2, p1]).await;
        let mut sts = Vec::with_capacity(vec.len());
        for v in vec.into_iter() {
            sts.push(v?);
        }

        Ok((c, sts))
    }
}

impl DatabaseService {
    pub(crate) async fn get_by_id_with_uid<T>(
        &self,
        st: &Statement,
        ids: &[u32],
    ) -> Result<(Vec<T>, Vec<u32>), ResError>
    where
        T: SelfUserId + SelfId + TryFrom<Row, Error = ResError>,
    {
        let (mut v, uids) = self
            .get_client()
            .query(st, &[&ids])
            .try_fold(
                (Vec::with_capacity(20), Vec::with_capacity(20)),
                move |(mut v, mut uids), r| {
                    if let Ok(r) = T::try_from(r) {
                        uids.push(r.get_user_id());
                        v.push(r)
                    }
                    futures::future::ok((v, uids))
                },
            )
            .await?;

        let mut result = Vec::with_capacity(v.len());
        for id in ids.iter() {
            for (i, idv) in v.iter().enumerate() {
                if id == &idv.self_id() {
                    result.push(v.swap_remove(i));
                    break;
                }
            }
        }

        Ok((result, uids))
    }
}

impl GetDbClient for DatabaseService {
    fn get_client(&self) -> RefMut<Client> {
        self.client.borrow_mut()
    }
}

impl Query for DatabaseService {}

impl SimpleQuery for DatabaseService {}

pub trait GetDbClient {
    fn get_client(&self) -> RefMut<Client>;
}

pub trait Query: GetDbClient {
    fn query_one_trait<T>(&self, st: &Statement, p: &[&dyn ToSql]) -> PinedBoxFutureResult<T>
    where
        T: TryFrom<Row, Error = ResError>,
    {
        Box::pin(
            self.get_client()
                .query(st, p)
                .try_collect::<Vec<Row>>()
                .map(|r| T::try_from(r?.pop().ok_or(ResError::BadRequest)?)),
        )
    }

    fn query_multi_trait<T>(
        &self,
        st: &Statement,
        p: &[&dyn ToSql],
        vec: Vec<T>,
    ) -> PinedBoxFutureResult<Vec<T>>
    where
        T: TryFrom<Row, Error = ResError> + 'static,
    {
        Box::pin(query_multi(&mut self.get_client(), st, p, vec))
    }
}

pub trait SimpleQuery: GetDbClient {
    fn simple_query_single_column_trait<T>(
        &self,
        query: &str,
        column_index: usize,
    ) -> PinedBoxFutureResult<T>
    where
        T: std::str::FromStr,
    {
        Box::pin(
            self.simple_query_row_trait(query)
                .map(move |r| parse_one_simple_row_column(r, column_index)),
        )
    }

    fn simple_query_one_trait<T>(&self, query: &str) -> PinedBoxFutureResult<T>
    where
        T: TryFrom<SimpleQueryRow, Error = ResError>,
    {
        Box::pin(self.simple_query_row_trait(query).map(|r| T::try_from(r?)))
    }

    fn simple_query_row_trait(&self, q: &str) -> PinedBoxFutureResult<SimpleQueryRow> {
        Box::pin(
            self.get_client()
                .simple_query(q)
                .try_collect::<Vec<SimpleQueryMessage>>()
                .map(|r| pop_simple_message(r?)),
        )
    }
}

fn parse_one_simple_row_column<T>(
    result: Result<SimpleQueryRow, ResError>,
    column_index: usize,
) -> Result<T, ResError>
where
    T: std::str::FromStr,
{
    result?
        .get(column_index)
        .ok_or(ResError::DataBaseReadError)?
        .parse::<T>()
        .map_err(|_| ResError::ParseError)
}

fn pop_simple_message(mut vec: Vec<SimpleQueryMessage>) -> Result<SimpleQueryRow, ResError> {
    vec.pop();
    match vec.pop().ok_or(ResError::BadRequest)? {
        SimpleQueryMessage::Row(r) => Ok(r),
        _ => pop_simple_message(vec),
    }
}

// helper functions for build cache on startup
/// when folding stream into data struct the error from parsing column are ignored.
/// We send all the good data to frontend.
pub fn query_multi<T>(
    client: &mut Client,
    st: &Statement,
    params: &[&dyn ToSql],
    vec: Vec<T>,
) -> impl Future<Output = Result<Vec<T>, ResError>>
where
    T: TryFrom<Row, Error = ResError> + 'static,
{
    client
        .query(st, params)
        .map_err(ResError::from)
        .try_fold(vec, move |mut vec, r| {
            if let Ok(r) = T::try_from(r) {
                vec.push(r);
            }
            futures::future::ok(vec)
        })
}

pub fn simple_query_one_column<T>(
    c: &mut Client,
    query: &str,
    index: usize,
) -> impl Future<Output = Result<T, ResError>>
where
    T: std::str::FromStr,
{
    c.simple_query(query)
        .try_collect::<Vec<SimpleQueryMessage>>()
        .map(|r| pop_simple_message(r?))
        .map(move |r| parse_one_simple_row_column(r, index))
}
