use actix::prelude::*;

use tokio::runtime::current_thread::Runtime;
use tokio_postgres::{connect, Client, tls::NoTls, Statement, SimpleQueryMessage};

use crate::handler::{
    db::get_all_categories,
    cache::{build_hash_set, build_list, update_meta, build_hmset},
    category::load_all_categories,
    post::{load_all_posts_with_topic_id, get_last_pid},
    topic::get_topic_list,
    user::{get_last_uid, load_all_users},
};
use crate::model::{
    common::{PostgresPool, RedisPool, GlobalVar, GlobalGuard},
};

// ToDo: build category ranks and topic ranks at startup;
pub fn build_cache(db_pool: &PostgresPool, cache_pool: &RedisPool, postgres_url: &str , redis_url: &str) -> Result<(), ()> {
    let conn = &db_pool.get().unwrap_or_else(|_| panic!("Database is offline"));
    let conn_cache = cache_pool.get().unwrap_or_else(|_| panic!("Cache is offline"));

    let mut rt: Runtime = Runtime::new().unwrap();
    let (mut c, conn_new) = rt.block_on(connect(postgres_url, NoTls)).unwrap_or_else(|_| panic!("Can't connect to db"));
    let mut c_cache = redis::Client::open(redis_url).unwrap_or_else(|_| panic!("Can't connect to cache"));

    rt.handle().spawn(conn_new.map_err(|e| panic!("{}", e))).unwrap();

    // Load all categories and make hash set.
    let p = c.prepare("SELECT * FROM categories");
    let st = rt.block_on(p).unwrap();
    let mut vec = Vec::new();
    let categories = rt.block_on(get_all_categories(&mut c, &st, vec)).unwrap();

    rt.block_on(build_hmset(&mut c_cache, categories.clone(), "category".to_owned())).unwrap_or_else(|_| panic!("Failed to update categories hash set"));

//    build_hash_set(&categories, "category", &conn_cache).unwrap_or_else(|_| panic!("Failed to update categories hash set"));

    /// build list by last reply time desc order for each category. build category meta list with all category ids
    let mut meta_ids = Vec::new();
    for cat in categories.iter() {
        meta_ids.push(cat.id);
        let topic_list = get_topic_list(&cat.id, conn).unwrap_or_else(|_| panic!("Failed to build category lists"));
        build_list(topic_list, &format!("category:{}", &cat.id), &conn_cache).unwrap_or_else(|_| panic!("Failed to build category lists"));
    }
    update_meta(meta_ids, "category_id", &conn_cache).unwrap_or_else(|_| panic!("Failed to build category meta"));

    /// Load all posts with topic id and build a list of posts for each topic
    let posts = load_all_posts_with_topic_id(&conn).unwrap_or_else(|_| panic!("Failed to load posts"));
    let mut temp = Vec::new();
    let mut index: u32 = posts[0].0;
    for post in posts.into_iter() {
        let (i, v) = post;
        if i == index {
            temp.push(v)
        } else {
            build_list(temp, &format!("topic:{}", index), &conn_cache).unwrap_or_else(|_| panic!("Failed to build category lists"));
            temp = Vec::new();
            index = i;
            temp.push(v);
        }
    }
    build_list(temp, &format!("topic:{}", index), &conn_cache).unwrap_or_else(|_| panic!("Failed to build category lists"));

    /// load all users and store the data in a zrange. stringify user data as member, user id as score.
    let users = load_all_users(conn).unwrap_or_else(|_| panic!("Failed to load users"));
    build_hash_set(&users, "user", &conn_cache).unwrap_or_else(|_| panic!("Failed to update users cache"));

    /// load all users talk rooms and store the data in a zrange. stringify user rooms and privilege as member, user id as score.

    Ok(())
}

pub fn generate_global(database_url: &str) -> GlobalGuard {
    let (u, p, t) = load_global(database_url);
    GlobalVar::new(u, p, t)
}


fn load_global(postgres_url: &str) -> (u32, u32, u32) {
    let cids: Vec<u32> = Vec::new();
    let tids: Vec<u32> = Vec::new();
    let uids: Vec<u32> = Vec::new();
    let pids: Vec<u32> = Vec::new();

    let mut rt: Runtime = Runtime::new().unwrap();

    let (mut c, conn) = rt.block_on(connect(postgres_url, NoTls)).unwrap();
    rt.handle().spawn(conn.map_err(|e| panic!("{}", e))).unwrap();

    let p = c.prepare("SELECT id FROM categories");
    let st = rt.block_on(p).unwrap();
    let f = c
        .query(&st, &[])
        .map_err(|e| panic!("{}", e))
        .fold(cids, move |mut cids, row| {
            cids.push(row.get(0));
            Ok::<_, _>(cids)
        });
    let mut cids = rt.block_on(f).unwrap();

    let mut query = String::new();
    for id in cids.iter() {
        query.push_str(&format!("SELECT id FROM topics{};", id))
    }
    let f = c
        .simple_query(&query)
        .map_err(|e| panic!("{}", e))
        .fold(tids, |mut tids, msg| {
            match msg {
                SimpleQueryMessage::Row(row) => tids.push(row
                    .get(0)
                    .map(|i| i
                        .parse::<u32>()
                        .unwrap_or(1))
                    .unwrap_or(1)
                ),
                _ => ()
            }
            Ok::<_, _>(tids)
        });
    let mut tids = rt.block_on(f).unwrap();

    let p = c.prepare("SELECT id FROM users ORDER BY id ASC");
    let st = rt.block_on(p).unwrap();
    let f = c
        .query(&st, &[])
        .map_err(|e| panic!("{}", e))
        .fold(uids, move |mut uids, row| {
            uids.push(row.get(0));
            Ok::<_, _>(uids)
        });
    let mut uids = rt.block_on(f).unwrap();

    let p = c.prepare("SELECT id FROM posts ORDER BY id ASC");
    let st = rt.block_on(p).unwrap();
    let f = c
        .query(&st, &[])
        .map_err(|e| panic!("{}", e))
        .fold(pids, move |mut pids, row| {
            pids.push(row.get(0));
            Ok::<_, _>(pids)
        });
    let mut pids = rt.block_on(f).unwrap();

    uids.sort();
    tids.sort();
    pids.sort();

    (uids.pop().unwrap_or(1),
     pids.pop().unwrap_or(1),
     tids.pop().unwrap_or(1))
}