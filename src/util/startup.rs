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
pub fn build_cache(postgres_url: &str, redis_url: &str) -> Result<GlobalGuard, ()> {
    let mut rt = Runtime::new().unwrap();
    let (mut c, conn) = rt.block_on(connect(postgres_url, NoTls)).unwrap_or_else(|_| panic!("Can't connect to db"));
    let c_cache = redis::Client::open(redis_url).unwrap_or_else(|_| panic!("Can't connect to cache"));
    let c_cache = rt.block_on(c_cache.get_shared_async_connection()).unwrap_or_else(|_| panic!("Can't get connection from redis"));

    rt.spawn(conn.map_err(|e| panic!("{}", e)));

    // Load all categories and make hash set.
    let p = c.prepare("SELECT * FROM categories");
    let st = rt.block_on(p).unwrap();
    let categories = Vec::new();
    let categories = rt.block_on(get_all_categories(&mut c, &st, categories)).unwrap();

    rt.block_on(build_hmset(c_cache.clone(), categories.clone(), "category")).unwrap_or_else(|_| panic!("Failed to update categories hash set"));

    // build list by last reply time desc order for each category. build category meta list with all category ids


    let mut last_tid = 1;
    let mut category_ids = Vec::new();
    for cat in categories.iter() {
        category_ids.push(cat.id);
        let query = format!("SELECT * FROM topics{} ORDER BY last_reply_time DESC", cat.id);
        let tids = Vec::new();
        let topics = Vec::new();
        let (topics, _) = rt.block_on(get_topics(&mut c, &query, topics, tids)).unwrap_or_else(|_| panic!("Failed to build category lists"));
        let tids = topics.into_iter().map(|t| {
            if t.id > last_tid { last_tid = t.id };
            t.id
        }).collect();
        let key = format!("category:{}:list", &cat.id);
        let _ = rt.block_on(build_list(c_cache.clone(), tids, "rpush", key)).unwrap_or_else(|_| panic!("Failed to build category lists"));
    }

    let _ = rt.block_on(build_list(c_cache.clone(), category_ids, "rpush", "category_id:meta".to_owned())).unwrap_or_else(|_| panic!("Failed to build category lists"));

    // Load all posts with topic id and build a list of posts for each topic
    // ToDo: iter category_ids vec and get posts from every topics{i} table and update topic's reply count with last reply time, post's reply count with last reply time, category's topic,post count.


    let mut last_pid = 1;
    for cat in categories.iter() {
        let query = format!("SELECT topic_id, id FROM posts{} ORDER BY topic_id ASC, id ASC", cat.id);
        let posts: Vec<(u32, u32)> = Vec::new();
        let f = c
            .simple_query(&query)
            .map_err(|e| panic!("{}", e))
            .fold(posts, |mut posts, row| {
                match row {
                    SimpleQueryMessage::Row(row) => {
                        if let Some(tid) = row.get(0).unwrap().parse::<u32>().ok() {
                            if let Some(pid) = row.get(1).unwrap().parse::<u32>().ok() {
                                posts.push((tid, pid));
                            }
                        }
                    }
                    _ => ()
                }
                Ok(posts)
            });

        let posts = rt.block_on(f).unwrap_or_else(|_| panic!("Failed to load posts"));

        if posts.len() > 0 {
            let mut temp = Vec::new();
            let mut index: u32 = posts[0].0;
            for post in posts.into_iter() {
                let (i, v) = post;

                if v > last_pid { last_pid = v };

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
        }
    }

    let p = c.prepare("SELECT * FROM users");
    let st = rt.block_on(p).unwrap();
    let users = rt.block_on(get_users_all(&mut c, &st)).unwrap_or_else(|_| panic!("Failed to load users"));

    // ToDo： collect all subscribe data from users and update category subscribe count.

    let mut last_uid = 1;
    for u in users.iter() {
        if u.id > last_uid { last_uid = u.id };
    }
    rt.block_on(build_hmset(c_cache.clone(), users, "user")).unwrap_or_else(|_| panic!("Failed to update categories hash set"));


    // ToDo: load all users talk rooms and store the data in a zrange. stringify user rooms and privilege as member, user id as score.


    Ok(GlobalVar::new(last_uid, last_pid, last_tid))
}