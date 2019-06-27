use actix::prelude::*;
use actix_rt::Runtime;
use tokio_postgres::{connect, Client, tls::NoTls, Statement, SimpleQueryMessage};

use crate::handler::{
    db::{get_all_categories, query_topics_simple, get_users_all, simple_query},
    cache::{build_list, build_hmset},
};
use crate::model::{
    common::{
        GlobalVar,
        GlobalGuard,
    }
};
use crate::handler::cache::{build_category_set, build_topic_set};
use chrono::NaiveDateTime;

//return global arc after building cache
pub fn build_cache(postgres_url: &str, redis_url: &str) -> Result<GlobalGuard, ()> {
    let mut rt = Runtime::new().unwrap();
    let (mut c, conn) = rt.block_on(connect(postgres_url, NoTls)).unwrap_or_else(|_| panic!("Can't connect to db"));
    rt.spawn(conn.map_err(|e| panic!("{}", e)));

    let c_cache = redis::Client::open(redis_url).unwrap_or_else(|_| panic!("Can't connect to cache"));
    let c_cache = rt.block_on(c_cache.get_shared_async_connection()).unwrap_or_else(|_| panic!("Can't get connection from redis"));

    // Load all categories and make hash set.
    let categories = rt.block_on(get_all_categories(&mut c)).unwrap();

    rt.block_on(build_hmset(c_cache.clone(), categories.clone(), "category")).unwrap_or_else(|_| panic!("Failed to update categories hash set"));

    // build list by last reply time desc order for each category. build category meta list with all category ids

    let mut last_tid = 1;
    let mut category_ids = Vec::new();
    for cat in categories.iter() {
        category_ids.push(cat.id);
        let query = format!("SELECT * FROM topics WHERE category_id = {} ORDER BY last_reply_time DESC", cat.id);
        let (t, _) = rt.block_on(query_topics_simple(&mut c, &query)).unwrap_or_else(|_| panic!("Failed to build category lists"));

        let mut tids = Vec::new();
        let mut sets = Vec::new();
        for t in t.clone().into_iter() {
            tids.push(t.id);
            sets.push((t.id, t.category_id, t.reply_count, t.last_reply_time));
            if t.id > last_tid { last_tid = t.id };
        }

        let _ = rt.block_on(build_category_set(sets, c_cache.clone())).unwrap_or_else(|_| panic!("Failed to build category sets"));

        let key = format!("category:{}:list", &cat.id);
        let _ = rt.block_on(build_list(c_cache.clone(), tids, "rpush", key)).unwrap_or_else(|_| panic!("Failed to build category lists"));
    }

    let _ = rt.block_on(build_list(c_cache.clone(), category_ids, "rpush", "category_id:meta".to_owned())).unwrap_or_else(|_| panic!("Failed to build category lists"));

    // Load all posts with topic id and build a list of posts for each topic
    // ToDo: iter category_ids vec and get posts from every topics{i} table and update topic's reply count with last reply time, post's reply count with last reply time, category's topic,post count.

    let mut last_pid = 1;
    let query = "SELECT topic_id, id, reply_count, last_reply_time FROM posts ORDER BY topic_id ASC, id ASC";
    let posts: Vec<(u32, u32, i32, NaiveDateTime)> = Vec::new();
    let f = c
        .simple_query(query)
        .map_err(|e| panic!("{}", e))
        .fold(posts, |mut posts, row| {
            match row {
                SimpleQueryMessage::Row(row) => {
                    if let Some(tid) = row.get(0).unwrap().parse::<u32>().ok() {
                        if let Some(pid) = row.get(1).unwrap().parse::<u32>().ok() {
                            if let Some(count) = row.get(2).unwrap().parse::<i32>().ok() {
                                if let Some(time) = NaiveDateTime::parse_from_str(row.get(3).unwrap(), "%Y-%m-%d %H:%M:%S%.f").ok() {
                                    posts.push((tid, pid, count, time));
                                }
                            }
                        }
                    }
                }
                _ => ()
            }
            Ok(posts)
        });

    let posts = rt.block_on(f).unwrap_or_else(|_| panic!("Failed to load posts"));

    if posts.len() > 0 {
        let _ = rt.block_on(build_topic_set(posts.clone(), c_cache.clone())).unwrap_or_else(|_| panic!("Failed to load posts"));

        let mut temp = Vec::new();
        let mut index: u32 = posts[0].0;
        for post in posts.into_iter() {
            let (tid, pid, _, _) = post;

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

    if let Some(_) = rt.block_on(get_all_categories(&mut c)).ok() {
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
    last_reply_time TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reply_count     INTEGER       NOT NULL DEFAULT 0,
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
    last_reply_time TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reply_count     INTEGER       NOT NULL DEFAULT 0,
    is_locked       BOOLEAN       NOT NULL DEFAULT FALSE
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
    privacy     OID          NOT NULL DEFAULT 0,
    owner       OID          NOT NULL,
    admin       OID[]        NOT NULL,
    users       OID[]        NOT NULL
);

CREATE TABLE public_messages1
(
    talk_id     OID          NOT NULL,
    time        TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    message     VARCHAR(1024)NOT NULL
);


CREATE INDEX pub_message_time_order ON public_messages1 (time DESC);
CREATE INDEX pub_message_belong ON public_messages1 (talk_id);

CREATE INDEX topic_time_order ON topics (last_reply_time DESC);
CREATE INDEX topic_reply_order ON topics (reply_count DESC);
CREATE INDEX topic_category_belong ON topics (category_id);
CREATE INDEX post_reply_order ON posts (reply_count DESC);
CREATE INDEX post_topic_belong ON posts (topic_id);
CREATE UNIQUE INDEX users_username ON users (username);
CREATE UNIQUE INDEX users_email ON users (email);
CREATE UNIQUE INDEX categories_name ON categories (name);
CREATE UNIQUE INDEX talks_name ON talks (name);
CREATE UNIQUE INDEX associates_psn_id ON associates (psn_id);
CREATE UNIQUE INDEX associates_live_id ON associates (live_id);

CREATE OR REPLACE FUNCTION adding_topic() RETURNS trigger AS
$adding_topic$
BEGIN
    IF EXISTS(SELECT id FROM categories WHERE id = NEW.category_id)
    THEN
        UPDATE categories
        SET topic_count = topic_count + 1
        WHERE id = NEW.category_id;
        RETURN NEW;
    ELSE RETURN NULL;
    END IF;
END;
$adding_topic$ LANGUAGE plpgsql;

CREATE TRIGGER adding_topic
    BEFORE INSERT
    ON topics
    FOR EACH ROW
EXECUTE PROCEDURE adding_topic();

CREATE OR REPLACE FUNCTION adding_post() RETURNS trigger AS
$adding_post$
BEGIN
    IF NOT EXISTS(SELECT id FROM topics WHERE id = NEW.topic_id)
    THEN
        RETURN NULL;
    END IF;

    IF NEW.post_id IS NOT NULL AND EXISTS(SELECT id FROM posts WHERE id = NEW.post_id AND topic_id = NEW.topic_id)
    THEN
        UPDATE posts
        SET reply_count     = reply_count + 1,
            last_reply_time = NEW.created_at
        WHERE id = NEW.post_id;
    ELSE
        NEW.post_id = NULL;
    END IF;

    UPDATE categories
    SET post_count = post_count + 1
    WHERE id = NEW.category_id;

    UPDATE topics
    SET reply_count     = reply_count + 1,
        last_reply_time = NEW.created_at
    WHERE id = NEW.topic_id;

    RETURN NEW;
END;
$adding_post$ LANGUAGE plpgsql;

CREATE TRIGGER adding_post
    BEFORE INSERT
    ON posts
    FOR EACH ROW
EXECUTE PROCEDURE adding_post();".to_owned();

    // insert dummy data.default adminuser password is 1234asdf
    query.push_str("
INSERT INTO users (id, username, email, hashed_password, signature, avatar_url, is_admin)
VALUES (1, 'adminuser', 'admin@pixelshare', '$2y$06$z6K5TMA2TQbls77he7cEsOQQ4ekgCNvuxkg6eSKdHHLO9u6sY9d3C', 'AdminUser',
        'avatar_url', 9);

INSERT INTO categories (id, name, thumbnail)
VALUES (1, 'General', 'category_default.png');

INSERT INTO categories (id, name, thumbnail)
VALUES (2, 'Announcement', 'category_default.png'),
       (3, 'Armored Core', 'ac.jpg'),
       (4, 'Ace Combat', 'ace.jpg'),
       (5, 'Persona', 'persona.jpg');

INSERT INTO talks (id, name, description, owner, admin, users)
VALUES (1, 'test123', 'test123', 1, ARRAY [1, 2, 3], ARRAY [1, 2, 3]);

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

DROP TRIGGER IF EXISTS adding_post ON posts;
DROP FUNCTION IF EXISTS adding_post();
DROP TRIGGER IF EXISTS adding_topic ON topics;
DROP FUNCTION IF EXISTS adding_topic();

DROP TABLE IF EXISTS topics;
DROP TABLE IF EXISTS posts;";

    let _ = rt.block_on(simple_query(&mut c, query).and_then(|_| {
        println!("All tables have been drop. pixel_rs exited");
        Ok(())
    })).expect("failed to clear db");
}