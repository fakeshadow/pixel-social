use actix::{Actor, SyncContext};
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};

use std::sync::{Arc};
use redis::Client;

pub struct DbExecutor(pub Pool<ConnectionManager<PgConnection>>);

impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}

pub struct CacheExecutor(pub Arc<Client>);

impl Actor for CacheExecutor {
    type Context = SyncContext<Self>;
}
