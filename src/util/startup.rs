use chrono::NaiveDateTime;
use futures::FutureExt;
use redis::aio::SharedConnection;
use tokio_postgres::{tls::NoTls, types::ToSql, Client, SimpleQueryMessage};

use crate::handler::{
    cache::{
        build_hmsets_fn, build_list, build_posts_cache_list, build_topics_cache_list,
        build_users_cache,
    },
    db::ParseRowStream,
};
use crate::model::talk::Talk;
use crate::model::{
    category::Category,
    common::{GLOBALS, TALKS},
    errors::ResError,
    topic::Topic,
    user::User,
};

const BUILD_TABLES: &str = "
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
CREATE UNIQUE INDEX associates_live_id ON associates (live_id);


CREATE TABLE psn_user_trophy_titles
(
np_id                   VARCHAR(32)         NOT NULL,
np_communication_id     VARCHAR(32)         NOT NULL,
is_visible              BOOLEAN             NOT NULL DEFAULT TRUE,
progress                OID                 NOT NULL DEFAULT 0,
earned_platinum         OID                 NOT NULL DEFAULT 0,
earned_gold             OID                 NOT NULL DEFAULT 0,
earned_silver           OID                 NOT NULL DEFAULT 0,
earned_bronze           OID                 NOT NULL DEFAULT 0,
last_update_date        TIMESTAMP           NOT NULL
);

CREATE TABLE trophy_sets
(
np_communication_id     VARCHAR(32)         NOT NULL,
trophy_id               OID                 NOT NULL,
trophy_hidden           BOOLEAN             NOT NULL,
trophy_type             VARCHAR(16)         NOT NULL,
trophy_name             VARCHAR(128)        NOT NULL,
trophy_detail           VARCHAR(1024)       NOT NULL,
trophy_icon_url         VARCHAR(1024)       NOT NULL,
trophy_rare             INTEGER             NOT NULL,
trophy_earned_rate      VARCHAR(16)         NOT NULL
);

CREATE TYPE should_before_after AS
(
trophy_id               OID,
reason                  VARCHAR(1024),
agreement               OID,
disagreement            OID
);

CREATE TYPE should_absent_time AS
(
beginning               TIMESTAMP,
ending                  TIMESTAMP,
is_regular              BOOLEAN,
reason                  VARCHAR(1024),
agreement               OID,
disagreement            OID
);

CREATE TABLE trophy_sets_argument
(
np_communication_id     VARCHAR(32)         NOT NULL,
trophy_id               INTEGER             NOT NULL,
should_before           should_before_after[],
should_after            should_before_after[],
should_absent_time      should_absent_time[]
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
VALUES (1, 1, 1, 1, 'First Reply Only to stop cache build from complaining');
";

const DROP_TABLES: &str = "
DROP TABLE IF EXISTS associates;
DROP TABLE IF EXISTS talks;
DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS categories;
DROP TABLE IF EXISTS public_messages1;
DROP TABLE IF EXISTS private_messages1;
DROP TABLE IF EXISTS relations;

DROP TABLE IF EXISTS psn_user_trophy_titles;
DROP TABLE IF EXISTS psn_user_trophy_sets;
DROP TABLE IF EXISTS trophy_sets;
DROP TABLE IF EXISTS trophy_sets_argument;

DROP TYPE IF EXISTS trophy_set;
DROP TYPE IF EXISTS should_before_after;
DROP TYPE IF EXISTS should_absent_time;

DROP TABLE IF EXISTS topics;
DROP TABLE IF EXISTS posts;";

//return global arc after building cache
pub async fn build_cache(
    postgres_url: &str,
    redis_url: &str,
    is_init: bool,
) -> Result<(), ResError> {
    let (c, conn) = tokio_postgres::connect(postgres_url, NoTls).await?;

    tokio::spawn(conn.map(|_| ()));

    let c_cache = &mut redis::Client::open(redis_url)
        .unwrap_or_else(|e| panic!("{}", e))
        .get_shared_async_connection()
        .await?;

    // Load all categories and make hash map sets.
    let categories = build_categories_cache(&c, c_cache).await?;

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
        let query = format!(
            "SELECT COUNT(id) FROM topics WHERE category_id = {}",
            cat.id
        );
        let t_count = crate::handler::db::simple_query_one_column::<u32>(&c, query.as_str(), 0)
            .await
            .unwrap_or(0);

        let query = format!("SELECT COUNT(id) FROM posts WHERE category_id = {}", cat.id);
        let p_count = crate::handler::db::simple_query_one_column::<u32>(&c, query.as_str(), 0)
            .await
            .unwrap_or(0);

        redis::cmd("HMSET")
            .arg(&format!("category:{}:set", cat.id))
            .arg(&[
                ("topic_count", t_count.to_string()),
                ("post_count", p_count.to_string()),
            ])
            .query_async::<_, ()>(c_cache)
            .await?;

        // ToDo: don't update popular list for categories by created_at order. Use set_perm key and last_reply_time field instead.
        // load topics belong to category
        let st = c
            .prepare("SELECT * FROM topics WHERE category_id = $1 ORDER BY created_at DESC")
            .await?;

        let params: [&(dyn ToSql + Sync); 1] = [&cat.id];

        let t = c
            .query_raw(&st, params.iter().map(|s| *s as &dyn ToSql))
            .await?
            .parse_row::<Topic>()
            .await?;

        // load topics reply count
        let query = format!(
            "SELECT COUNT(topic_id), topic_id FROM posts WHERE category_id = {} GROUP BY topic_id",
            cat.id
        );
        let rows = c.simple_query(query.as_str()).await?;

        let mut reply_count = Vec::new();
        for row in rows.into_iter() {
            if let SimpleQueryMessage::Row(row) = row {
                if let Ok(count) = row.get(0).unwrap().parse::<u32>() {
                    if let Ok(tid) = row.get(1).unwrap().parse::<u32>() {
                        reply_count.push((tid, count));
                    }
                }
            }
        }

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

        build_topics_cache_list(is_init, sets, c_cache).await?;
    }

    build_list(c_cache, category_ids, "category_id:meta".to_owned()).await?;

    // load all posts with tid id and created_at
    let rows = c
        .simple_query("SELECT topic_id, id, created_at FROM posts")
        .await?;

    let mut posts = Vec::new();
    for row in rows.into_iter() {
        if let SimpleQueryMessage::Row(row) = row {
            let tid = row.get(0).unwrap().parse::<u32>().unwrap();
            let pid = row.get(1).unwrap().parse::<u32>().unwrap();
            let time =
                NaiveDateTime::parse_from_str(row.get(2).unwrap(), "%Y-%m-%d %H:%M:%S%.f").unwrap();
            posts.push((tid, pid, None, time));
        }
    }

    // load topics reply count
    let rows = c
        .simple_query("SELECT COUNT(post_id), post_id FROM posts GROUP BY post_id")
        .await?;

    let mut reply_count = Vec::new();
    for row in rows.into_iter() {
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
    }

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
        let _ = build_posts_cache_list(is_init, posts, c_cache).await;
    }

    let last_uid = build_users_cache_local(&c, c_cache).await?;

    let st = c.prepare("SELECT * FROM talks").await?;
    let params: [&(dyn ToSql + Sync); 0] = [];
    let t = c
        .query_raw(&st, params.iter().map(|s| *s as &dyn ToSql))
        .await?
        .parse_row::<Talk>()
        .await?;

    let mut talks = TALKS.0.write().await;

    for t in t.into_iter() {
        talks.insert(t.id, t);
    }

    // ToDo: load all users talk rooms and store the data in a zrange. stringify user rooms and privilege as member, user id as score.

    GLOBALS
        .lock()
        .await
        .update(last_uid, last_pid, last_tid, last_cid);

    Ok(())
}

async fn build_categories_cache(
    c: &Client,
    c_cache: &mut SharedConnection,
) -> Result<Vec<Category>, ResError> {
    let st = c.prepare_typed("SELECT * FROM categories", &[]).await?;
    let params: [&(dyn ToSql + Sync); 0] = [];
    let categories = c
        .query_raw(&st, params.iter().map(|s| *s as &dyn ToSql))
        .await?
        .parse_row::<Category>()
        .await?;

    build_hmsets_fn(
        c_cache,
        &categories,
        crate::handler::cache::CATEGORY_U8,
        false,
    )
    .await?;

    Ok(categories)
}

// return last user.id in result for building global vars.
async fn build_users_cache_local(
    c: &Client,
    c_cache: &mut SharedConnection,
) -> Result<u32, ResError> {
    let st = c.prepare("SELECT * FROM users").await?;
    let params: [&(dyn ToSql + Sync); 0] = [];

    let users = c
        .query_raw(&st, params.iter().map(|s| *s as &dyn ToSql))
        .await?
        .parse_row::<User>()
        .await?;

    // ToDo： collect all subscribe data from users and update category subscribe count.

    let mut last_uid = 1;
    for u in users.iter() {
        if u.id > last_uid {
            last_uid = u.id
        };
    }

    build_users_cache(users, c_cache).await?;

    Ok(last_uid)
}

// return Ok(false) if tables already exist.
async fn create_table(postgres_url: &str) -> Result<bool, ResError> {
    let (c, conn) = tokio_postgres::connect(postgres_url, NoTls).await?;

    tokio::spawn(conn.map(|_| ()));

    let st = c.prepare("SELECT * FROM categories").await;

    if st.is_ok() {
        return Ok(false);
    }

    c.simple_query(BUILD_TABLES).await?;

    Ok(true)
}

async fn drop_table(postgres_url: &str) -> Result<(), ResError> {
    let (c, conn) = tokio_postgres::connect(postgres_url, NoTls).await?;

    tokio::spawn(conn.map(|_| ()));

    c.simple_query(DROP_TABLES).await?;
    Ok(())
}

pub(crate) async fn init_table_cache(args: &[String], postgres_url: &str, redis_url: &str) -> bool {
    let mut is_init = false;
    for arg in args.iter() {
        if arg == "drop" {
            drop_table(&postgres_url)
                .await
                .unwrap_or_else(|e| panic!("{}", e));

            let _ = crate::handler::cache::clear_cache(&redis_url);

            std::process::exit(1);
        }
        if arg == "build" {
            let success = create_table(&postgres_url)
                .await
                .unwrap_or_else(|e| panic!("{}", e));
            if success {
                is_init = true;
            } else {
                println!("tables already exists. building cache with is_init = false");
            }
        }
    }
    is_init
}
