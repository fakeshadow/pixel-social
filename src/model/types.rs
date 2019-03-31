use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool as diesel_pool};

use r2d2_redis::{redis, r2d2 as redis_r2d2, RedisConnectionManager};

pub type PostgresPool = diesel_pool<ConnectionManager<PgConnection>>;
pub type RedisPool = redis_r2d2::Pool<RedisConnectionManager>;