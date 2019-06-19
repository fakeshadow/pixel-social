use actix::prelude::*;
use actix_rt::Runtime;
use tokio_postgres::{connect, Client, tls::NoTls, Statement, SimpleQueryMessage};

use crate::handler::{
    db::{get_all_categories, get_topics, get_users_all},
    cache::{build_list, build_hmset},
};
use crate::model::{
    common::{GlobalVar, GlobalGuard},
};

// ToDo: combine global var generate with build cache process;
pub fn build_cache(postgres_url: &str, redis_url: &str) -> Result<(), ()> {

    let mut rt = Runtime::new().unwrap();
    let (mut c, conn) = rt.block_on(connect(postgres_url, NoTls)).unwrap_or_else(|_| panic!("Can't connect to db"));
    let mut c_cache = redis::Client::open(redis_url).unwrap_or_else(|_| panic!("Can't connect to cache"));
    let c_cache = rt.block_on(c_cache.get_shared_async_connection()).unwrap_or_else(|_| panic!("Can't get connection from redis"));

    rt.spawn(conn.map_err(|e| panic!("{}", e)));

    // Load all categories and make hash set.
    let p = c.prepare("SELECT * FROM categories");
    let st = rt.block_on(p).unwrap();
    let categories = Vec::new();
    let categories = rt.block_on(get_all_categories(&mut c, &st, categories)).unwrap();



    rt.block_on(build_hmset(c_cache.clone(), categories.clone(), "category")).unwrap_or_else(|_| panic!("Failed to update categories hash set"));

    // build list by last reply time desc order for each category. build category meta list with all category ids
    let mut category_ids = Vec::new();
    for cat in categories.iter() {
        category_ids.push(cat.id);
        let query = format!("SELECT * FROM topics{} ORDER BY last_reply_time DESC", cat.id);
        let tids = Vec::new();
        let topics = Vec::new();
        let (topics, _) = rt.block_on(get_topics(&mut c, &query, topics, tids)).unwrap_or_else(|_| panic!("Failed to build category lists"));
        let tids = topics.into_iter().map(|t| t.id).collect();
        let key = format!("category:{}:list", &cat.id);
        let _ = rt.block_on(build_list(c_cache.clone(), tids, "rpush", key)).unwrap_or_else(|_| panic!("Failed to build category lists"));
    }

    let _ = rt.block_on(build_list(c_cache.clone(), category_ids, "rpush", "category_id:meta".to_owned())).unwrap_or_else(|_| panic!("Failed to build category lists"));

    // Load all posts with topic id and build a list of posts for each topic
    // ToDo: iter category_ids vec and get posts from every topics{i} table and update topic's reply count with last reply time, post's reply count with last reply time, category's topic,post count.
//    let posts = load_all_posts_with_topic_id(&conn).unwrap_or_else(|_| panic!("Failed to load posts"));

    let p = c.prepare("SELECT topic_id, id FROM posts ORDER BY topic_id ASC, id ASC");
    let st = rt.block_on(p).unwrap();

    let posts: Vec<(u32, u32)> = Vec::new();
    let f = c
        .query(&st, &[])
        .map_err(|e| panic!("{}", e))
        .fold(posts, |mut posts, row| {
            posts.push((row.get(0), row.get(1)));
            Ok(posts)
        });

    let posts = rt.block_on(f).unwrap_or_else(|_| panic!("Failed to load posts"));

    let mut temp = Vec::new();
    let mut index: u32 = posts[0].0;
    for post in posts.into_iter() {
        let (i, v) = post;
        if i == index {
            temp.push(v)
        } else {
            let key = format!("topic:{}:list", &index);
            let _ = rt.block_on(build_list(c_cache.clone(), temp, "rpush", key)).unwrap_or_else(|_| panic!("Failed to build topic lists"));
            temp = Vec::new();
            index = i;
            temp.push(v);
        }
    }
    let key = format!("topic:{}:list", &index);
    let _ = rt.block_on(build_list(c_cache.clone(), temp, "rpush", key)).unwrap_or_else(|_| panic!("Failed to build topic lists"));


    let p = c.prepare("SELECT * FROM users");
    let st = rt.block_on(p).unwrap();

    let users = rt.block_on(get_users_all(&mut c, &st)).unwrap_or_else(|_| panic!("Failed to load users"));
    // ToDo： collect all subscribe data from users and update category subscribe count.
    rt.block_on(build_hmset(c_cache.clone(), users, "user")).unwrap_or_else(|_| panic!("Failed to update categories hash set"));

    // load all users talk rooms and store the data in a zrange. stringify user rooms and privilege as member, user id as score.
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
    rt.spawn(conn.map_err(|e| panic!("{}", e)));

    let p = c.prepare("SELECT id FROM categories");
    let st = rt.block_on(p).unwrap();
    let f = c
        .query(&st, &[])
        .map_err(|e| panic!("{}", e))
        .fold(cids, move |mut cids, row| {
            cids.push(row.get(0));
            Ok::<_, _>(cids)
        });
    let cids = rt.block_on(f).unwrap();

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