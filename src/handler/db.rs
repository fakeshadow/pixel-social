use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Stream, TryFutureExt};
use once_cell::sync::OnceCell;
use tokio_postgres::{types::Type, Client, NoTls, Row, SimpleQueryMessage, Statement};
use tokio_postgres_tang::{Builder, Pool, PoolRef, PostgresManager};

use crate::model::{common::SelfUserId, db_schema::TryFromRow, errors::ResError};

// frequent used statements that are construct on start.
const SELECT_TOPIC: &str = "SELECT * FROM topics WHERE id=ANY($1)";
const SELECT_POST: &str = "SELECT * FROM posts WHERE id=ANY($1)";
const SELECT_USER: &str = "SELECT * FROM users WHERE id=ANY($1)";
const INSERT_PUB_MSG: &str =
    "INSERT INTO public_messages1 (talk_id, text, time) VALUES ($1, $2, $3)";
const INSERT_PRV_MSG: &str =
    "INSERT INTO private_messages1 (from_id, to_id, text, time) VALUES ($1, $2, $3, $4)";

// construct static postgres pool so that it can be freely used through out the app without cloning.

pub fn pool() -> &'static MyPostgresPool {
    static POOL: OnceCell<MyPostgresPool> = OnceCell::new();

    POOL.get_or_init(|| {
        MyPostgresPool::new(
            std::env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set in .env")
                .as_str(),
        )
    })
}

#[derive(Clone)]
pub struct MyPostgresPool(Pool<PostgresManager<NoTls>>);

impl MyPostgresPool {
    pub(crate) fn new(postgres_url: &str) -> MyPostgresPool {
        let mgr = PostgresManager::new_from_stringlike(postgres_url, NoTls)
            .expect("Failed to create postgres pool manager")
            .prepare_statement("topics_by_id", SELECT_TOPIC, &[Type::OID_ARRAY])
            .prepare_statement("posts_by_id", SELECT_POST, &[Type::OID_ARRAY])
            .prepare_statement("users_by_id", SELECT_USER, &[Type::OID_ARRAY])
            .prepare_statement("insert_pub_msg", INSERT_PUB_MSG, &[])
            .prepare_statement("insert_prv_msg", INSERT_PRV_MSG, &[]);

        let pool = Builder::new()
            .always_check(false)
            .idle_timeout(None)
            .max_lifetime(None)
            .min_idle(1)
            .max_size(12)
            .build_uninitialized(mgr);

        MyPostgresPool(pool)
    }

    pub(crate) async fn init(&self) {
        self.0
            .init()
            .await
            .expect("Failed to initialize postgres pool");
    }

    pub(crate) fn get(
        &self,
    ) -> impl Future<Output = Result<PoolRef<'_, PostgresManager<NoTls>>, ResError>> {
        self.0.get().err_into()
    }
}

// helper functions for build cache on startup
pub(crate) async fn simple_query_one_column<T>(
    c: &Client,
    query: &str,
    column_index: usize,
) -> Result<T, ResError>
where
    T: std::str::FromStr,
{
    c.simple_query(query)
        .await?
        .first()
        .ok_or(ResError::PostgresError)
        .map(|msg| match msg {
            SimpleQueryMessage::Row(r) => Ok(r),
            _ => Err(ResError::BadRequest),
        })??
        .get(column_index)
        .ok_or(ResError::PostgresError)?
        .parse::<T>()
        .map_err(|_| ResError::ParseError)
}

// trait for parsing RowStream into Vec<T> we want. T have to impl TryFromRow trait in crate::model::db_schema
// this trait is generic because RowStream is not a public API.
pub trait ParseRowStream<S>: Sized {
    fn parse_row<R>(self) -> RowStreamFuture<R, S>;

    fn parse_row_with<R>(self) -> RowStreamFutureWith<R, S>;
}

pub struct RowStreamFuture<R, S> {
    stream: S,
    items: Vec<R>,
}

pub struct RowStreamFutureWith<R, S> {
    stream: S,
    items: (Vec<R>, Vec<u32>),
}

impl<S> ParseRowStream<S> for S {
    fn parse_row<R>(self) -> RowStreamFuture<R, S> {
        RowStreamFuture {
            stream: self,
            items: Vec::with_capacity(21),
        }
    }

    fn parse_row_with<R>(self) -> RowStreamFutureWith<R, S> {
        RowStreamFutureWith {
            stream: self,
            items: (Vec::with_capacity(21), Vec::with_capacity(21)),
        }
    }
}

// Use the same implementation as futures::TryStream::try_collect. So it should be safe.
impl<R, S> RowStreamFuture<R, S> {
    fn stream(self: Pin<&mut Self>) -> Pin<&mut S> {
        unsafe { std::pin::Pin::map_unchecked_mut(self, |x| &mut x.stream) }
    }

    fn items(self: Pin<&mut Self>) -> &mut Vec<R> {
        unsafe { &mut std::pin::Pin::get_unchecked_mut(self).items }
    }
}

// Use the same implementation as futures::TryStream::try_collect. So it should be safe.
impl<R, S> RowStreamFutureWith<R, S> {
    fn stream(self: Pin<&mut Self>) -> Pin<&mut S> {
        unsafe { std::pin::Pin::map_unchecked_mut(self, |x| &mut x.stream) }
    }

    fn items(self: Pin<&mut Self>) -> &mut (Vec<R>, Vec<u32>) {
        unsafe { &mut std::pin::Pin::get_unchecked_mut(self).items }
    }
}

impl<R, S> Future for RowStreamFuture<R, S>
where
    R: TryFromRow<Row, Error = ResError>,
    S: Stream<Item = Result<Row, tokio_postgres::Error>>,
{
    type Output = Result<Vec<R>, ResError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match futures::ready!(self.as_mut().stream().poll_next(cx)) {
                Some(result) => match result {
                    Ok(row) => {
                        if let Ok(t) = R::try_from_row(&row) {
                            self.as_mut().items().push(t);
                        }
                    }
                    Err(e) => return Poll::Ready(Err(e.into())),
                },
                None => {
                    return Poll::Ready(Ok(std::mem::replace(self.as_mut().items(), Vec::new())));
                }
            }
        }
    }
}

impl<R, S> Future for RowStreamFutureWith<R, S>
where
    R: TryFromRow<Row, Error = ResError> + SelfUserId,
    S: Stream<Item = Result<Row, tokio_postgres::Error>>,
{
    type Output = Result<(Vec<R>, Vec<u32>), ResError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match futures::ready!(self.as_mut().stream().poll_next(cx)) {
                Some(result) => match result {
                    Ok(row) => {
                        if let Ok(t) = R::try_from_row(&row) {
                            let items = self.as_mut().items();
                            let uid = t.get_user_id();
                            if !items.1.contains(&uid) {
                                items.1.push(uid);
                            }
                            items.0.push(t);
                        }
                    }
                    Err(e) => return Poll::Ready(Err(e.into())),
                },
                None => {
                    return Poll::Ready(Ok(std::mem::replace(
                        self.as_mut().items(),
                        (Vec::new(), Vec::new()),
                    )));
                }
            }
        }
    }
}

pub trait GetStatement {
    fn get_statement(&self, alias: &str) -> Result<&Statement, ResError>;
}

impl GetStatement for std::collections::HashMap<String, Statement> {
    fn get_statement(&self, alias: &str) -> Result<&Statement, ResError> {
        self.get(alias).ok_or(ResError::PostgresError)
    }
}
