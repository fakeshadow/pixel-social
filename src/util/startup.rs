
use actix::SystemRunner;
use chrono::NaiveDateTime;
use futures::{ FutureExt, TryFutureExt, TryStreamExt};
use futures01::Future as Future01;
use tokio_postgres::{connect, SimpleQueryMessage, tls::NoTls};

use crate::handler::{
    cache::{
        build_hmsets_01, build_list, build_posts_cache_list_01, build_topics_cache_list_01, build_users_cache_01,
    },
    db::load_all_to_01,
};
use crate::model::{
    category::Category,
    common::{
        GlobalSessions,
        GlobalTalks,
        GlobalVar, GlobalVars,
        new_global_talks_sessions,
    },
    topic::Topic,
    user::User,
};
use crate::model::talk::Talk;

//return global arc after building cache
pub fn build_cache(
    sys: &mut SystemRunner,
    postgres_url: &str,
    redis_url: &str,
    is_init: bool,
) -> Result<(GlobalVars, GlobalTalks, GlobalSessions), ()> {
    let (mut c, connection) = sys.block_on(connect(postgres_url, NoTls).boxed().compat()).unwrap_or_else(|e| panic!("{}", e));
    let connection = connection.map(|_| ());

    actix::spawn(connection.unit_error().boxed().compat());

    let c_cache = sys.block_on(redis::Client::open(redis_url).unwrap_or_else(|e| panic!("{}", e))
        .get_shared_async_connection())
        .unwrap_or_else(|e| panic!("{}", e));

    // Load all categories and make hash map sets.
    let query = "SELECT * FROM categories";

    let categories = sys.block_on(load_all_to_01::<Category>(&mut c, query)).unwrap_or_else(|_| panic!("Failed to load categories"));

    let _ = sys.block_on(build_hmsets_01(
        c_cache.clone(),
        &categories,
        crate::handler::cache::CATEGORY_U8,
        false)).unwrap_or_else(|_| panic!("Failed to update categories sets"));

    // build list by create_time desc order for each category. build category meta list with all category ids

    let mut last_tid = 1;
    let mut last_cid = 1;
    let mut category_ids = Vec::new();
    for cat in categories.iter() {
        if cat.id > last_cid {
            last_cid = cat.id
        };
        category_ids.push(cat.id);

        // count posts and topics for each category and write to redis
        let query = format!("SELECT COUNT(id) FROM topics WHERE category_id = {}", cat.id);
        let t_count = sys.block_on(crate::handler::db::simple_query_single_row_handler_to_01::<u32>(&mut c, query.as_str(), 0)).unwrap_or(0);

        let query = format!("SELECT COUNT(id) FROM posts WHERE category_id = {}", cat.id);
        let p_count = sys.block_on(crate::handler::db::simple_query_single_row_handler_to_01::<u32>(&mut c, query.as_str(), 0)).unwrap_or(0);

        let _ = sys.block_on(
            redis::cmd("HMSET")
                .arg(&format!("category:{}:set", cat.id))
                .arg(&[
                    ("topic_count", t_count.to_string()),
                    ("post_count", p_count.to_string()),
                ])
                .query_async(c_cache.clone())
                .map(|(_, ())| ())
        ).unwrap_or_else(|_| panic!("Failed to build category post/topic count"));

        // ToDo: don't update popular list for categories by created_at order. Use set_perm key and last_reply_time field instead.
        // load topics belong to category
        let query = format!(
            "SELECT * FROM topics WHERE category_id = {} ORDER BY created_at DESC",
            cat.id
        );
        let t = sys.block_on(crate::handler::db::load_all_to_01::<Topic>(&mut c, &query))
            .unwrap_or_else(|_| panic!("Failed to build category lists"));

        // load topics reply count
        let query = format!(
            "SELECT COUNT(topic_id), topic_id FROM posts WHERE category_id = {} GROUP BY topic_id",
            cat.id
        );
        let reply_count = Vec::new();
        let f = c.simple_query(&query)
            .try_fold(reply_count, |mut reply_count, row| {
                if let SimpleQueryMessage::Row(row) = row {
                    if let Ok(count) = row.get(0).unwrap().parse::<u32>() {
                        if let Ok(tid) = row.get(1).unwrap().parse::<u32>() {
                            reply_count.push((tid, count));
                        }
                    }
                }
                futures::future::ok(reply_count)
            },
            );

        let mut reply_count: Vec<(u32, u32)> = sys.block_on(f.boxed().compat()).unwrap_or_else(|_| panic!("Failed to get topics reply count"));

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
        for t in t.into_iter() {
            tids.push(t.id);
            sets.push((t.id, t.category_id, t.reply_count, t.created_at));
            if t.id > last_tid {
                last_tid = t.id
            };
        }

        sys.block_on(build_topics_cache_list_01(is_init, sets, c_cache.clone()))
            .unwrap_or_else(|_| panic!("Failed to build category sets"));
    }
    sys.block_on(build_list(
        c_cache.clone(),
        category_ids,
        "category_id:meta".to_owned()))
        .unwrap_or_else(|_| panic!("Failed to build category lists"));

    // load all posts with tid id and created_at
    let f = c
        .simple_query("SELECT topic_id, id, created_at FROM posts")
        .map_err(|e| panic!("{}", e))
        .try_fold(Vec::new(), |mut posts, row| {
            if let SimpleQueryMessage::Row(row) = row {
                let tid = row.get(0).unwrap().parse::<u32>().unwrap();
                let pid = row.get(1).unwrap().parse::<u32>().unwrap();
                let time =
                    NaiveDateTime::parse_from_str(row.get(2).unwrap(), "%Y-%m-%d %H:%M:%S%.f")
                        .unwrap();
                posts.push((tid, pid, None, time));
            }
            futures::future::ok(posts)
        });

    let posts = sys
        .block_on(f.boxed().compat())
        .unwrap_or_else(|_| panic!("Failed to load posts"));

    // load topics reply count
    let f = c
        .simple_query("SELECT COUNT(post_id), post_id FROM posts GROUP BY post_id")
        .map_err(|e| panic!("{}", e))
        .try_fold(Vec::new(), |mut reply_count, row| {
            if let SimpleQueryMessage::Row(row) = row {
                if let Some(str) = row.get(0) {
                    if let Ok(count) = str.parse::<u32>() {
                        if let Some(str) = row.get(1) {
                            if let Ok(pid) = str.parse::<u32>() {
                                reply_count.push((pid, count));
                            }
                        }
                    }
                }
            }
            futures::future::ok(reply_count)
        });

    let mut reply_count: Vec<(u32, u32)> = sys.block_on(f.boxed().compat()).unwrap_or_else(|_| panic!("Failed to get topics reply count"));

    let mut last_pid = 1;

    // attach reply count to posts
    let posts = posts
        .into_iter()
        .map(|mut p| {
            if p.1 > last_pid {
                last_pid = p.1;
            }

            for i in 0..reply_count.len() {
                if p.1 == reply_count[i].0 {
                    p.2 = Some(reply_count[i].1);
                    reply_count.remove(i);
                    break;
                }
            }
            p
        })
        .collect::<Vec<(u32, u32, Option<u32>, NaiveDateTime)>>();

    if !posts.is_empty() {
        sys.block_on(build_posts_cache_list_01(is_init, posts, c_cache.clone()))
            .unwrap_or_else(|_| panic!("Failed to build posts cache"));
    }

    let users = sys.block_on(crate::handler::db::load_all_to_01::<User>(&mut c, "SELECT * FROM users"))
        .unwrap_or_else(|_| panic!("Failed to load users"));

    // ToDo： collect all subscribe data from users and update category subscribe count.

    let mut last_uid = 1;
    for u in users.iter() {
        if u.id > last_uid {
            last_uid = u.id
        };
    }
    sys.block_on(build_users_cache_01(users, c_cache.clone()))
        .unwrap_or_else(|_| panic!("Failed to build users cache"));

    let talks = sys.block_on(load_all_to_01::<Talk>(&mut c, "SELECT * FROM talks"))
        .unwrap_or_else(|_| panic!("Failed to load talks"));

    let (talks, sessions) = new_global_talks_sessions(talks);

    // ToDo: load all users talk rooms and store the data in a zrange. stringify user rooms and privilege as member, user id as score.

    Ok((
        GlobalVar::new(last_uid, last_pid, last_tid, last_cid),
        talks,
        sessions,
    ))
}

// return true if built tables success
pub fn create_table(sys: &mut SystemRunner, postgres_url: &str) -> bool {
    let (mut c, connection) = sys.block_on(connect(postgres_url, NoTls).boxed().compat()).unwrap_or_else(|e| panic!("{}", e));
    let connection = connection.map(|_| ());
    actix::spawn(connection.unit_error().boxed().compat());

    let query = "SELECT * FROM categories";

    if sys.block_on(crate::handler::db::load_all_to_01::<Category>(&mut c, query))
        .ok()
        .is_some()
    {
        return false;
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
    name             VARCHAR(128) NOT NULL UNIQUE,
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
    is_locked       BOOLEAN       NOT NULL DEFAULT FALSE,
    is_visible      BOOLEAN       NOT NULL DEFAULT TRUE
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
    id              OID             NOT NULL UNIQUE PRIMARY KEY,
    name            VARCHAR(128)    NOT NULL UNIQUE,
    description     VARCHAR(128)    NOT NULL,
    secret          VARCHAR(128)    NOT NULL DEFAULT '1',
    privacy         OID             NOT NULL DEFAULT 0,
    owner           OID             NOT NULL,
    admin           OID[]           NOT NULL,
    users           OID[]           NOT NULL
);

CREATE TABLE relations
(
    id          OID             NOT NULL UNIQUE PRIMARY KEY,
    friends     OID[]
);

CREATE TABLE public_messages1
(
    talk_id     OID             NOT NULL PRIMARY KEY,
    time        TIMESTAMP       NOT NULL DEFAULT CURRENT_TIMESTAMP,
    text        VARCHAR(1024)   NOT NULL
);

CREATE TABLE private_messages1
(
    from_id     OID             NOT NULL,
    to_id       OID             NOT NULL PRIMARY KEY,
    time        TIMESTAMP       NOT NULL DEFAULT CURRENT_TIMESTAMP,
    text        VARCHAR(1024)   NOT NULL
);

CREATE INDEX pub_message_time_order ON public_messages1 (time DESC);
CREATE INDEX prv_message_time_order ON private_messages1 (time DESC);

CREATE UNIQUE INDEX users_username ON users (username);
CREATE UNIQUE INDEX users_email ON users (email);
CREATE UNIQUE INDEX categories_name ON categories (name);
CREATE UNIQUE INDEX talks_name ON talks (name);
CREATE UNIQUE INDEX associates_psn_id ON associates (psn_id);
CREATE UNIQUE INDEX associates_live_id ON associates (live_id);"
        .to_owned();

    // create table for PSN data.
    query.push_str(
        "
CREATE TABLE psn_user_trophy_titles
(
    np_id                   VARCHAR(32)         NOT NULL,
    np_communication_id     VARCHAR(32)         NOT NULL,
    is_visible              BOOLEAN             NOT NULL DEFAULT TRUE,
    progress                INTEGER             NOT NULL DEFAULT 0,
    earned_platinum         INTEGER             NOT NULL DEFAULT 0,
    earned_gold             INTEGER             NOT NULL DEFAULT 0,
    earned_silver           INTEGER             NOT NULL DEFAULT 0,
    earned_bronze           INTEGER             NOT NULL DEFAULT 0,
    last_update_date        TIMESTAMP           NOT NULL
);

CREATE TYPE trophy_set AS
(
    trophy_id               INTEGER,
    earned_date             TIMESTAMP,
    first_earned_date       TIMESTAMP
);

CREATE TABLE psn_user_trophy_sets
(
    np_id                   VARCHAR(32)         NOT NULL,
    np_communication_id     VARCHAR(32)         NOT NULL,
    is_visible              BOOLEAN             NOT NULL DEFAULT TRUE,
    trophy_set              trophy_set[]
);

CREATE UNIQUE INDEX user_trophy_titles ON psn_user_trophy_titles (np_id, np_communication_id);
CREATE UNIQUE INDEX user_trophy_sets ON psn_user_trophy_sets (np_id, np_communication_id);
",
    );

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

    let f = c.simple_query(&query).try_collect::<Vec<_>>();

    let _ = sys.block_on(f.boxed().compat()).unwrap_or_else(|_| panic! {"failed to create tables"});

    true
}

pub fn drop_table(sys: &mut SystemRunner, postgres_url: &str) {
    let (mut c, conn) = sys.block_on(connect(postgres_url, NoTls).boxed().compat()).unwrap_or_else(|e| panic!("{}", e));
    let conn = conn.map(|_| ());
    actix::spawn(conn.unit_error().boxed().compat());

    let query = "
DROP TABLE IF EXISTS associates;
DROP TABLE IF EXISTS talks;
DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS categories;
DROP TABLE IF EXISTS public_messages1;
DROP TABLE IF EXISTS private_messages1;
DROP TABLE IF EXISTS relations;

DROP TABLE IF EXISTS psn_user_trophy_titles;
DROP TABLE IF EXISTS psn_user_trophy_sets;
DROP TYPE IF EXISTS trophy_set;

DROP TABLE IF EXISTS topics;
DROP TABLE IF EXISTS posts;";

    let f = c.simple_query(query).try_collect::<Vec<_>>();
    let _ = sys.block_on(f.boxed().compat()).unwrap_or_else(|e| panic!("{}", e));
}
