use std::future::Future;

use futures::{FutureExt, TryFutureExt, TryStreamExt};
use tang_rs::{Builder, Pool, PoolRef, PostgresManager};
use tokio_postgres::{
    types::{ToSql, Type},
    Client, NoTls, Row, SimpleQueryMessage, SimpleQueryRow, Statement,
};

use crate::model::{
    common::{SelfId, SelfUserId},
    db_schema::TryFromRef,
    errors::ResError,
};

// frequent used statements that are construct on start.
const SELECT_TOPIC: &str = "SELECT * FROM topics WHERE id=ANY($1)";
const SELECT_POST: &str = "SELECT * FROM posts WHERE id=ANY($1)";
const SELECT_USER: &str = "SELECT * FROM users WHERE id=ANY($1)";

const INSERT_PUB_MSG: &str =
    "INSERT INTO public_messages1 (talk_id, text, time) VALUES ($1, $2, $3)";
const INSERT_PRV_MSG: &str =
    "INSERT INTO private_messages1 (from_id, to_id, text, time) VALUES ($1, $2, $3, $4)";

#[derive(Clone)]
pub struct MyPostgresPool(Pool<PostgresManager<NoTls>>);

impl MyPostgresPool {
    pub(crate) async fn new(postgres_url: &str) -> MyPostgresPool {
        let statements = vec![
            (SELECT_TOPIC, vec![Type::OID_ARRAY]),
            (SELECT_POST, vec![Type::OID_ARRAY]),
            (SELECT_USER, vec![Type::OID_ARRAY]),
            (INSERT_PUB_MSG, vec![]),
            (INSERT_PRV_MSG, vec![]),
        ];

        let mgr = PostgresManager::new_from_stringlike(postgres_url, statements, NoTls)
            .expect("Failed to create postgres pool manager");

        let pool = Builder::new()
            .always_check(false)
            .idle_timeout(None)
            .max_lifetime(None)
            .min_idle(36)
            .max_size(36)
            .build(mgr)
            .await
            .expect("Failed to build postgres pool");

        MyPostgresPool(pool)
    }

    pub(crate) fn get_pool(&self) -> impl Future<Output = Result<MyPoolRef, ResError>> {
        self.0.get().err_into().map_ok(MyPoolRef)
    }
}

pub struct MyPoolRef<'a>(PoolRef<'a, PostgresManager<NoTls>>);

impl MyPoolRef<'_> {
    pub fn get_client(&mut self) -> CratePostgresClient {
        CratePostgresClient(&mut self.0.get_conn().0)
    }

    pub fn get_client_statements(&mut self) -> (CratePostgresClient, CratePostgresStatements) {
        let (client, statements) = self.0.get_conn();

        (
            CratePostgresClient(client),
            CratePostgresStatements(statements),
        )
    }
}

pub struct CratePostgresClient<'a>(&'a mut Client);

pub struct CratePostgresStatements<'a>(&'a mut [Statement]);

impl<'a> CratePostgresStatements<'a> {
    pub(crate) fn get_statement(&self, index: usize) -> Result<&Statement, ResError> {
        self.0.get(index).ok_or(ResError::BadRequest)
    }
}

impl<'a> CratePostgresClient<'a> {
    pub(crate) fn execute(
        &mut self,
        st: &Statement,
        p: &[&(dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<u64, ResError>> {
        self.0.execute(st, p).err_into()
    }

    pub(crate) fn prepare(
        &mut self,
        query: &str,
    ) -> impl Future<Output = Result<Statement, ResError>> {
        self.0.prepare(query).err_into()
    }

    pub(crate) fn prepare_typed(
        &mut self,
        query: &str,
        types: &[Type],
    ) -> impl Future<Output = Result<Statement, ResError>> {
        self.0.prepare_typed(query, types).err_into()
    }

    pub(crate) fn query_multi_with_uid<T>(
        &mut self,
        st: &Statement,
        ids: &[u32],
    ) -> impl Future<Output = Result<(Vec<T>, Vec<u32>), ResError>>
    where
        T: SelfUserId + SelfId + TryFromRef<Row, Error = ResError> + Unpin,
    {
        self.0.query(st, &[&ids]).err_into().try_fold(
            (Vec::with_capacity(20), Vec::with_capacity(20)),
            move |(mut v, mut uids), r| {
                if let Ok(r) = T::try_from_ref(&r) {
                    uids.push(r.get_user_id());
                    v.push(r)
                }
                futures::future::ok((v, uids))
            },
        )
    }

    pub(crate) async fn query_one<T>(
        &mut self,
        st: &Statement,
        p: &[&(dyn ToSql + Sync)],
    ) -> Result<T, ResError>
    where
        T: TryFromRef<Row, Error = ResError> + Send,
    {
        Box::pin(self.0.query(st, p))
            .try_next()
            .map(|r| T::try_from_ref(&r?.ok_or(ResError::BadRequest)?))
            .await
    }

    pub(crate) fn query_multi<T>(
        &mut self,
        st: &Statement,
        p: &[&(dyn ToSql + Sync)],
        vec: Vec<T>,
    ) -> impl Future<Output = Result<Vec<T>, ResError>> + Send
    where
        T: TryFromRef<Row, Error = ResError> + Send,
    {
        query_multi_fn(self.0, st, p, vec)
    }

    pub(crate) fn simple_query_row(
        &'a mut self,
        query: &'a str,
    ) -> impl Future<Output = Result<SimpleQueryRow, ResError>> + Send + 'a {
        simple_query_row_fn(self.0, query)
    }
}

async fn simple_query_row_fn(c: &mut Client, query: &str) -> Result<SimpleQueryRow, ResError> {
    Box::pin(c.simple_query(query))
        .try_next()
        .map(|r| match r?.ok_or(ResError::BadRequest)? {
            SimpleQueryMessage::Row(r) => Ok(r),
            _ => Err(ResError::BadRequest),
        })
        .await
}

// helper functions for build cache on startup
/// when folding stream into data struct the error from parsing column are ignored.
/// We send all the good data to frontend.
pub(crate) fn query_multi_fn<T>(
    client: &mut Client,
    st: &Statement,
    params: &[&(dyn ToSql + Sync)],
    vec: Vec<T>,
) -> impl Future<Output = Result<Vec<T>, ResError>>
where
    T: TryFromRef<Row, Error = ResError>,
{
    client
        .query(st, params)
        .err_into()
        .try_fold(vec, move |mut vec, r| {
            if let Ok(r) = T::try_from_ref(&r) {
                vec.push(r);
            }
            futures::future::ok(vec)
        })
}

pub(crate) async fn simple_query_one_column<T>(
    c: &mut Client,
    query: &str,
    column_index: usize,
) -> Result<T, ResError>
where
    T: std::str::FromStr,
{
    simple_query_row_fn(c, query)
        .await?
        .get(column_index)
        .ok_or(ResError::DataBaseReadError)?
        .parse::<T>()
        .map_err(|_| ResError::ParseError)
}
