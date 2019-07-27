use actix::prelude::*;
use actix_rt::Runtime;

use tokio_postgres::{connect, tls::NoTls, SimpleQueryMessage};

use crate::handler::{
    cache::{build_list, build_hmsets, build_topics_cache_list, build_posts_cache_list},
};
use crate::model::{
    actors::DatabaseService,
    topic::Topic,
    user::User,
    talk::Talk,
    category::Category,
    common::{
        GlobalTalks,
        GlobalSessions,
        new_global_talks_sessions,
        GlobalVar,
        GlobalVars,
    },
};

//return global arc after building cache
pub fn build_cache(postgres_url: &str, redis_url: &str, is_init: bool) -> Result<(GlobalVars, GlobalTalks, GlobalSessions), ()> {
    let mut rt = Runtime::new().unwrap();
    let (mut c, conn) = rt.block_on(connect(postgres_url, NoTls)).unwrap_or_else(|_| panic!("Can't connect to db"));
    rt.spawn(conn.map_err(|e| panic!("{}", e)));

    let c_cache = redis::Client::open(redis_url).unwrap_or_else(|_| panic!("Can't connect to cache"));
    let c_cache = rt.block_on(c_cache.get_shared_async_connection()).unwrap_or_else(|_| panic!("Can't get connection from redis"));

    // Load all categories and make hash map sets.
    let query = "SELECT * FROM categories";
    let categories = rt.block_on(DatabaseService::query_multi_simple_no_limit::<Category>(&mut c, query, None)).unwrap();
    rt.block_on(build_hmsets(c_cache.clone(), categories.clone(), "category", false)).unwrap_or_else(|_| panic!("Failed to update categories sets"));


    // build list by create_time desc order for each category. build category meta list with all category ids

    let mut last_tid = 1;
    let mut category_ids = Vec::new();
    for cat in categories.iter() {
        category_ids.push(cat.id);

        // count posts and topics for each category and write to redis
        let query = format!("SELECT COUNT(id) FROM topics WHERE category_id = {}", cat.id);
        let f = DatabaseService::query_single_row::<u32>(&mut c, query.as_str(), 0, None);
        let t_count = rt.block_on(f).unwrap_or(0);

        let query = format!("SELECT COUNT(id) FROM posts WHERE category_id = {}", cat.id);
        let f = DatabaseService::query_single_row::<u32>(&mut c, query.as_str(), 0, None);
        let p_count = rt.block_on(f).unwrap_or(0);

        let f = redis::cmd("HMSET")
            .arg(&format!("category:{}:set", cat.id))
            .arg(&[("topic_count", t_count.to_string()), ("post_count", p_count.to_string())])
            .query_async(c_cache.clone())
            .map(|(_, ())| ());
        rt.block_on(f).unwrap_or_else(|_| panic!("Failed to build category post/topic count"));

        // ToDo: don't update popular list for categories by created_at order. Use set_perm key and last_reply_time field instead.
        // load topics belong to category
        let query = format!("SELECT * FROM topics WHERE category_id = {} ORDER BY created_at DESC", cat.id);
        let t: Vec<Topic> = rt.block_on(DatabaseService::query_multi_simple_no_limit(&mut c, &query, None))
            .unwrap_or_else(|_| panic!("Failed to build category lists"));

        // load topics reply count
        let query = format!("SELECT COUNT(topic_id), topic_id FROM posts WHERE category_id = {} GROUP BY topic_id", cat.id);
        let reply_count = Vec::new();
        let f = c.simple_query(&query)
            .map_err(|e| panic!("{}", e))
            .fold(reply_count, |mut reply_count, row| {
                match row {
                    SimpleQueryMessage::Row(row) => {
                        if let Some(count) = row.get(0).unwrap().parse::<u32>().ok() {
                            if let Some(tid) = row.get(1).unwrap().parse::<u32>().ok() {
                                reply_count.push((tid, count));
                            }
                        }
                    }
                    _ => ()
                }
                Ok(reply_count)
            });

        let mut reply_count: Vec<(u32, u32)> = rt.block_on(f).unwrap_or_else(|_| panic!("Failed to get topics reply count"));

        // attach reply count to topics
        let t = t
            .into_iter()
            .map(|mut t| {
                for i in 0..reply_count.len() {
                    if t.id == reply_count[i].0 {
                        t.reply_count = Some(reply_count[i].1);
                        reply_count.remove(i);
                        break;
                    }
                }
                t
            })
            .collect::<Vec<Topic>>();

        // build topics cache list.
        let mut tids = Vec::new();
        let mut sets = Vec::new();
        for t in t.clone().into_iter() {
            tids.push(t.id);
            sets.push((t.id, t.category_id, t.reply_count.unwrap_or(0), t.created_at));
            if t.id > last_tid { last_tid = t.id };
        }

        let _ = rt.block_on(build_topics_cache_list(is_init, sets, c_cache.clone())).unwrap_or_else(|_| panic!("Failed to build category sets"));
        if is_init {
            let key = format!("category:{}:list", &cat.id);
            let _ = rt.block_on(build_list(c_cache.clone(), tids, "rpush", key)).unwrap_or_else(|_| panic!("Failed to build category lists"));
        }
    }
    let _ = rt.block_on(build_list(c_cache.clone(), category_ids, "rpush", "category_id:meta".to_owned())).unwrap_or_else(|_| panic!("Failed to build category lists"));

    // Load all posts with topic id and build a list of posts for each topic
    let mut last_pid = 1;
    let query = "SELECT topic_id, id FROM posts ORDER BY topic_id ASC, id ASC";
    let posts: Vec<(u32, u32, u32)> = Vec::new();
    let f = c
        .simple_query(query)
        .map_err(|e| panic!("{}", e))
        .fold(posts, |mut posts, row| {
            match row {
                SimpleQueryMessage::Row(row) => {
                    if let Some(tid) = row.get(0).unwrap().parse::<u32>().ok() {
                        if let Some(pid) = row.get(1).unwrap().parse::<u32>().ok() {
                            posts.push((tid, pid, 0));
                        }
                    }
                }
                _ => ()
            }
            Ok(posts)
        });

    let posts = rt.block_on(f).unwrap_or_else(|_| panic!("Failed to load posts"));

    // load topics reply count
    let query = "SELECT COUNT(post_id), post_id FROM posts GROUP BY post_id";
    let reply_count = Vec::new();
    let f = c.simple_query(&query)
        .map_err(|e| panic!("{}", e))
        .fold(reply_count, |mut reply_count, row| {
            match row {
                SimpleQueryMessage::Row(row) => {
                    if let Some(str) = row.get(0) {
                        if let Some(count) = str.parse::<u32>().ok() {
                            if let Some(str) = row.get(1) {
                                if let Some(pid) = str.parse::<u32>().ok() {
                                    reply_count.push((pid, count));
                                }
                            }
                        }
                    }
                }
                _ => ()
            }
            Ok(reply_count)
        });

    let mut reply_count: Vec<(u32, u32)> = rt.block_on(f).unwrap_or_else(|_| panic!("Failed to get topics reply count"));

    // attach reply count to posts
    let posts = posts
        .into_iter()
        .map(|mut p| {
            for i in 0..reply_count.len() {
                if p.1 == reply_count[i].0 {
                    p.2 = reply_count[i].1;
                    reply_count.remove(i);
                    break;
                }
            }
            p
        })
        .collect::<Vec<(u32, u32, u32)>>();

    if posts.len() > 0 {
        let _ = rt.block_on(build_posts_cache_list(posts.clone(), c_cache.clone())).unwrap_or_else(|_| panic!("Failed to load posts"));

        let mut temp = Vec::new();
        let mut index: u32 = posts[0].0;
        for post in posts.into_iter() {
            let (tid, pid, _) = post;

            if pid > last_pid { last_pid = pid };

            if tid == index {
                temp.push(pid)
            } else {
                let key = format!("topic:{}:list", &index);
                let _ = rt.block_on(build_list(c_cache.clone(), temp, "rpush", key)).unwrap_or_else(|_| panic!("Failed to build topic lists"));
                temp = Vec::new();
                index = tid;
                temp.push(pid);
            }
        }
        let key = format!("topic:{}:list", &index);
        let _ = rt.block_on(build_list(c_cache.clone(), temp, "rpush", key)).unwrap_or_else(|_| panic!("Failed to build topic lists"));
    }

    let p = c.prepare("SELECT * FROM users");
    let st = rt.block_on(p).unwrap();
    let users = rt.block_on(DatabaseService::query_multi_no_limit::<User>(&mut c, &st, &[], None)).unwrap_or_else(|_| panic!("Failed to load users"));

    // ToDo： collect all subscribe data from users and update category subscribe count.

    let mut last_uid = 1;
    for u in users.iter() {
        if u.id > last_uid { last_uid = u.id };
    }
    rt.block_on(build_hmsets(c_cache.clone(), users, "user", false)).unwrap_or_else(|_| panic!("Failed to update categories hash set"));

    let p = c.prepare("SELECT * FROM talks");
    let st = rt.block_on(p).unwrap();

    let talks = rt.block_on(DatabaseService::query_multi_no_limit::<Talk>(&mut c, &st, &[], None)).unwrap_or_else(|_| panic!("Failed to load talks"));


    let (talks, sessions, ) = new_global_talks_sessions(talks);

    // ToDo: load all users talk rooms and store the data in a zrange. stringify user rooms and privilege as member, user id as score.

    Ok((GlobalVar::new(last_uid, last_pid, last_tid), talks, sessions))
}

pub fn create_table(postgres_url: &str) {
    let mut rt = Runtime::new().unwrap();
    let (mut c, conn) = rt.block_on(connect(postgres_url, NoTls)).unwrap_or_else(|_| panic!("Can't connect to db"));
    rt.spawn(conn.map_err(|e| panic!("{}", e)));

    let query = "SELECT * FROM categories";
    if let Some(_) = rt.block_on(DatabaseService::query_multi_simple_no_limit::<Category>(&mut c, query, None)).ok() {
        return;
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
    privilege       OID          NOT NULL DEFAULT 1,
    show_email      BOOLEAN      NOT NULL DEFAULT TRUE
);
CREATE TABLE categories
(
    id               OID          NOT NULL UNIQUE PRIMARY KEY,
    name             VARCHAR(128) NOT NULL,
    thumbnail        VARCHAR(256) NOT NULL
);
CREATE TABLE topics
(
    id              OID           NOT NULL UNIQUE PRIMARY KEY,
    user_id         OID           NOT NULL,
    category_id     OID           NOT NULL,
    title           VARCHAR(1024) NOT NULL,
    body            VARCHAR(1024) NOT NULL,
    thumbnail       VARCHAR(1024) NOT NULL,
    created_at      TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_locked       BOOLEAN       NOT NULL DEFAULT FALSE
);
CREATE TABLE posts
(
    id              OID           NOT NULL UNIQUE PRIMARY KEY,
    user_id         OID           NOT NULL,
    topic_id        OID           NOT NULL,
    category_id     OID           NOT NULL,
    post_id         OID,
    post_content    VARCHAR(1024) NOT NULL,
    created_at      TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_locked       BOOLEAN       NOT NULL DEFAULT FALSE
);
CREATE TABLE associates
(
    id               OID          NOT NULL UNIQUE PRIMARY KEY,
    user_id          OID          NOT NULL UNIQUE,
    psn_id           VARCHAR(128) UNIQUE,
    live_id          VARCHAR(128) UNIQUE,
    last_update_time TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE talks
(
    id          OID          NOT NULL UNIQUE PRIMARY KEY,
    name        VARCHAR(128) NOT NULL UNIQUE,
    description VARCHAR(128) NOT NULL,
    secret      VARCHAR(128) NOT NULL DEFAULT '1',
    privacy     OID          NOT NULL DEFAULT 0,
    owner       OID          NOT NULL,
    admin       OID[]        NOT NULL,
    users       OID[]        NOT NULL
);

CREATE TABLE relations
(
    id          OID          NOT NULL UNIQUE PRIMARY KEY,
    friends     OID[]
);

CREATE TABLE public_messages1
(
    talk_id     OID          NOT NULL PRIMARY KEY,
    time        TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    message     VARCHAR(1024)NOT NULL
);

CREATE TABLE private_messages1
(
    from_id     OID          NOT NULL,
    to_id       OID          NOT NULL PRIMARY KEY,
    time        TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    message     VARCHAR(1024)NOT NULL
);

CREATE INDEX pub_message_time_order ON public_messages1 (time DESC);
CREATE INDEX prv_message_time_order ON private_messages1 (time DESC);

CREATE UNIQUE INDEX users_username ON users (username);
CREATE UNIQUE INDEX users_email ON users (email);
CREATE UNIQUE INDEX categories_name ON categories (name);
CREATE UNIQUE INDEX talks_name ON talks (name);
CREATE UNIQUE INDEX associates_psn_id ON associates (psn_id);
CREATE UNIQUE INDEX associates_live_id ON associates (live_id);".to_owned();

    // insert dummy data.default adminuser password is 1234asdf
    query.push_str("
INSERT INTO users (id, username, email, hashed_password, signature, avatar_url, privilege)
VALUES (1, 'adminuser', 'admin@pixelshare', '$2y$06$z6K5TMA2TQbls77he7cEsOQQ4ekgCNvuxkg6eSKdHHLO9u6sY9d3C', 'AdminUser', 'ac.jpg', 9),
       (2, 'testtest1', 'test123@test123', '$2y$06$z6K5TMA2TQbls77he7cEsOQQ4ekgCNvuxkg6eSKdHHLO9u6sY9d3C', 'AdminUser', 'ac.jpg', 0),
       (3, 'testtest2', 'test223@test123', '$2y$06$z6K5TMA2TQbls77he7cEsOQQ4ekgCNvuxkg6eSKdHHLO9u6sY9d3C', 'AdminUser', 'ac.jpg', 1),
       (4, 'testtest3', 'test323@test123', '$2y$06$z6K5TMA2TQbls77he7cEsOQQ4ekgCNvuxkg6eSKdHHLO9u6sY9d3C', 'AdminUser', 'ac.jpg', 2);

INSERT INTO relations (id, friends)
VALUES (1, ARRAY[2,3,4]);

INSERT INTO categories (id, name, thumbnail)
VALUES (1, 'General', 'category_default.png');

INSERT INTO categories (id, name, thumbnail)
VALUES (2, 'Announcement', 'category_default.png'),
       (3, 'Armored Core', 'ac.jpg'),
       (4, 'Ace Combat', 'ace.jpg'),
       (5, 'Persona', 'persona.jpg');

INSERT INTO talks (id, name, description, owner, admin, users)
VALUES (1, 'general', 'ac.jpg', 1, ARRAY [1], ARRAY [1]),
       (2, 'special', 'ac.jpg', 1, ARRAY [1], ARRAY [1]),
       (3, 'test1', 'ac.jpg', 1, ARRAY [1], ARRAY [1]),
       (4, 'test2', 'ac.jpg', 1, ARRAY [1], ARRAY [1]),
       (5, 'test3', 'ac.jpg', 1, ARRAY [1], ARRAY [1]);

INSERT INTO topics (id, user_id, category_id, title, body, thumbnail)
VALUES (1, 1, 1, 'Welcome To PixelShare', 'PixelShare is a gaming oriented community.', '');

INSERT INTO posts (id, user_id, topic_id, category_id, post_content)
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

    let query = "
DROP TABLE IF EXISTS associates;
DROP TABLE IF EXISTS talks;
DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS categories;
DROP TABLE IF EXISTS public_messages1;
DROP TABLE IF EXISTS private_messages1;
DROP TABLE IF EXISTS relations;

DROP TRIGGER IF EXISTS adding_post ON posts;
DROP FUNCTION IF EXISTS adding_post();
DROP TRIGGER IF EXISTS adding_topic ON topics;
DROP FUNCTION IF EXISTS adding_topic();

DROP TABLE IF EXISTS topics;
DROP TABLE IF EXISTS posts;";

    let _ = rt.block_on(DatabaseService::simple_query(&mut c, query, None).and_then(|_| {
        println!("All tables have been drop. pixel_rs exited");
        Ok(())
    })).expect("failed to clear db");
}