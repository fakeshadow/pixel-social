use actix::prelude::*;
use actix_rt::Runtime;
use tokio_postgres::{connect, Client, tls::NoTls, Statement, SimpleQueryMessage};

use crate::handler::{
    db::{get_all_categories, query_topics, get_users_all, simple_query},
    cache::{build_list, build_hmset},
};
use crate::model::{
    common::{
        GlobalVar,
        GlobalGuard,
        create_topics_posts_table_sql,
    }
};

//return global arc after building cache
pub fn build_cache(postgres_url: &str, redis_url: &str) -> Result<GlobalGuard, ()> {
    let mut rt = Runtime::new().unwrap();
    let (mut c, conn) = rt.block_on(connect(postgres_url, NoTls)).unwrap_or_else(|_| panic!("Can't connect to db"));
    rt.spawn(conn.map_err(|e| panic!("{}", e)));

    let c_cache = redis::Client::open(redis_url).unwrap_or_else(|_| panic!("Can't connect to cache"));
    let c_cache = rt.block_on(c_cache.get_shared_async_connection()).unwrap_or_else(|_| panic!("Can't get connection from redis"));

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
        let (_, tids) = rt.block_on(query_topics(&mut c, &query)).unwrap_or_else(|_| panic!("Failed to build category lists"));

        for tid in tids.clone().into_iter() {
            if tid > last_tid { last_tid = tid };
        }

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

pub fn create_table(postgres_url: &str) {
    let mut rt = Runtime::new().unwrap();
    let (mut c, conn) = rt.block_on(connect(postgres_url, NoTls)).unwrap_or_else(|_| panic!("Can't connect to db"));
    rt.spawn(conn.map_err(|e| panic!("{}", e)));

    let p = c.prepare("SELECT * FROM categories");
    if let Some(st) = rt.block_on(p).ok() {
        let categories = Vec::new();
        if let Some(_) = rt.block_on(get_all_categories(&mut c, &st, categories)).ok() {
            return;
        }
    }
    // create default table
    let mut query = "
CREATE TABLE users
(
    id              OID          NOT NULL UNIQUE PRIMARY KEY,
    username        VARCHAR(32)  NOT NULL UNIQUE,
    email           VARCHAR(100) NOT NULL UNIQUE,
    hashed_password VARCHAR(64)  NOT NULL,
    avatar_url      VARCHAR(128) NOT NULL,
    signature       VARCHAR(256) NOT NULL,
    created_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_admin        OID          NOT NULL DEFAULT 0,
    blocked         BOOLEAN      NOT NULL DEFAULT FALSE,
    show_email      BOOLEAN      NOT NULL DEFAULT TRUE,
    show_created_at BOOLEAN      NOT NULL DEFAULT TRUE,
    show_updated_at BOOLEAN      NOT NULL DEFAULT TRUE
);
CREATE TABLE categories
(
    id               OID          NOT NULL UNIQUE PRIMARY KEY,
    name             VARCHAR(128) NOT NULL,
    topic_count      INTEGER      NOT NULL DEFAULT 0,
    post_count       INTEGER      NOT NULL DEFAULT 0,
    subscriber_count INTEGER      NOT NULL DEFAULT 0,
    thumbnail        VARCHAR(256) NOT NULL
);
CREATE TABLE associates
(
    id               OID       NOT NULL UNIQUE PRIMARY KEY,
    user_id          OID       NOT NULL UNIQUE,
    psn_id           VARCHAR(128) UNIQUE,
    live_id          VARCHAR(128) UNIQUE,
    last_update_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE talks
(
    id          OID          NOT NULL UNIQUE PRIMARY KEY,
    name        VARCHAR(128) NOT NULL UNIQUE,
    description VARCHAR(128) NOT NULL,
    secret      VARCHAR(128) NOT NULL DEFAULT '1',
    owner       OID          NOT NULL,
    admin       OID[]        NOT NULL,
    users       OID[]        NOT NULL
);
CREATE UNIQUE INDEX users_username ON users (username);
CREATE UNIQUE INDEX users_email ON users (email);
CREATE UNIQUE INDEX categories_name ON categories (name);
CREATE UNIQUE INDEX talks_name ON talks (name);
CREATE UNIQUE INDEX associates_psn_id ON associates (psn_id);
CREATE UNIQUE INDEX associates_live_id ON associates (live_id);
".to_owned();

    // create repeated default table
    for i in 1u32..6 {
        create_topics_posts_table_sql(&mut query, i);
    }

    // insert dummy data.default adminuser password is 1234asdf
    query.push_str("
INSERT INTO users (id, username, email, hashed_password, signature, avatar_url, is_admin)
VALUES (1, 'adminuser', 'admin@pixelshare', '$2y$06$z6K5TMA2TQbls77he7cEsOQQ4ekgCNvuxkg6eSKdHHLO9u6sY9d3C', 'AdminUser',
        'avatar_url', 9);

INSERT INTO categories (id, name, thumbnail, topic_count, post_count)
VALUES (1, 'General', 'category_default.png', 1, 1);

INSERT INTO categories (id, name, thumbnail)
VALUES (2, 'Announcement', 'category_default.png'),
       (3, 'Armored Core', 'ac.jpg'),
       (4, 'Ace Combat', 'ace.jpg'),
       (5, 'Persona', 'persona.jpg');

INSERT INTO talks (id, name, description, owner, admin, users)
VALUES (1, 'test123', 'test123', 1, ARRAY [1, 2, 3], ARRAY [1, 2, 3]);

INSERT INTO topics1 (id, user_id, category_id, reply_count, title, body, thumbnail)
VALUES (1, 1, 1, 1, 'Welcome To PixelShare', 'PixelShare is a gaming oriented community.', '');

INSERT INTO posts1 (id, user_id, topic_id, category_id, post_content)
VALUES (1, 1, 1, 1, 'First Reply Only to stop cache build from complaining');");

    let f = c.simple_query(&query).into_future();

    let _ = rt.block_on(f).and_then(|_| {
        println!("dummy tables generated");
        Ok(())
    }).unwrap_or_else(|_| panic!("fail to create default tables"));
}

pub fn drop_table(postgres_url: &str) {
    let mut rt = Runtime::new().unwrap();
    let (mut c, conn) = rt.block_on(connect(postgres_url, NoTls)).unwrap_or_else(|_| panic!("Can't connect to db"));
    rt.spawn(conn.map_err(|e| panic!("{}", e)));

    let p = c.prepare("SELECT * FROM categories");
    let st = rt.block_on(p).expect("failed to prepare statement");
    let categories = Vec::new();
    let categories = rt.block_on(get_all_categories(&mut c, &st, categories)).unwrap_or_else(|_| panic!("failed to get categories"));

    let mut query = "
DROP TABLE IF EXISTS associates;
DROP TABLE IF EXISTS talks;
DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS categories;".to_owned();

    for c in categories.iter() {
        let i = c.id;
        query.push_str(&format!("
DROP TRIGGER IF EXISTS adding_post{} ON posts{};
DROP FUNCTION IF EXISTS adding_post{}();
DROP TRIGGER IF EXISTS adding_topic{} ON topics{};
DROP FUNCTION IF EXISTS adding_topic{}();
DROP INDEX IF EXISTS topic_time_order{};
DROP INDEX IF EXISTS topic_reply_order{};
DROP INDEX IF EXISTS post_reply_order{};
DROP TABLE IF EXISTS topics{};
DROP TABLE IF EXISTS posts{};", i, i, i, i, i, i, i, i, i, i, i))
    }

    let _ = rt.block_on(simple_query(&mut c, &query).and_then(|_| {
        println!("All tables have been drop. pixel_rs exited");
        Ok(())
    })).expect("failed to clear db");
}