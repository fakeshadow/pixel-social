use diesel::{
    pg::PgConnection,
    prelude::*,
};
use r2d2_redis::redis;

use crate::model::{
    category::Category,
    errors::ServiceError,
    common::{GlobalVar, GlobalGuard, match_id}
};
//use crate::handler::cache;
use crate::schema::{posts, topics, users, categories};

pub fn clear_cache(redis_url: &str) -> Result<usize, ()> {
    let redis_client = redis::Client::open(redis_url).unwrap();
    let clear_cache = redis_client.get_connection().unwrap();
    redis::cmd("flushall").query(&clear_cache).map_err(|_| ())
}

struct CategoryHash<'a> {
    pub name: &'a str,
    pub thumbnail: &'a str
}

impl<'a> CategoryHash<'a> {
    fn new(name: &'a str, thumbnail: &'a str) -> Self {
        CategoryHash {
            name,
            thumbnail
        }
    }
}

// ToDo: Build category set, user set, topic rank at system start;
//pub fn build_cache(redis_url: &str, database_url: &str) -> Result<(),ServiceError> {
//    let conn = PgConnection::establish(database_url)
//        .unwrap_or_else(|_| panic!("Failed to connect to database"));
//
//    let categories: Vec<Category> = categories::table.find(()).load::<Category>(&conn)?;
//    for category in categories.iter() {
//        let id =category.id;
//        let name = category.name;
//        let topic_count = category.topic_count;
//        let post_count = category.post_count;
//        let subscriber_count = category.subscriber_count;
//        let thumbnail = category.thumbnail;
//        let hash_set_key = format!("category:{}:set",&id);
//        let category_range_key = format!("category:{}:rank", &id);
//
//    }
//
//    Ok(())
//}

pub fn init_global_var(database_url: &str) -> GlobalGuard {
    let connection = PgConnection::establish(database_url)
        .unwrap_or_else(|_| panic!("Failed to connect to database"));

    let last_uid = users::table.select(users::id).order(users::id.desc()).limit(1).load(&connection);
    let next_uid = match_id(last_uid);

    let last_pid = posts::table.select(posts::id).order(posts::id.desc()).limit(1).load(&connection);
    let next_pid = match_id(last_pid);

    let last_tid = topics::table.select(topics::id).order(topics::id.desc()).limit(1).load(&connection);
    let next_tid = match_id(last_tid);

    GlobalVar::new(next_uid, next_pid, next_tid)
}