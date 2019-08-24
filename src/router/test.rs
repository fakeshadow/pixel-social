use std::convert::TryFrom;

use actix::prelude::*;
use actix_web::{web::Data, Error, HttpResponse};

use crate::handler::cache::{CacheService, CheckCacheConn};
use crate::handler::db::DatabaseService;
use crate::model::errors::ResError;
use crate::model::topic::Topic;
use crate::model::{common::GlobalVars, post::PostRequest, topic::TopicRequest};

pub fn hello_world() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().json("hello world"))
}

pub fn add_topic(
    global: Data<GlobalVars>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let req = TopicRequest {
        id: None,
        user_id: Some(1),
        category_id: 1,
        thumbnail: Some("test thumbnail".to_string()),
        title: Some("test title".to_string()),
        body: Some("test body".to_string()),
        is_locked: None,
        is_visible: Some(true),
    };

    db.check_conn()
        .from_err()
        .and_then(move |opt| db.if_replace_db(opt).add_topic(&req, global.get_ref()))
        .from_err()
        .and_then(move |t| {
            cache.check_cache_conn().then(move |r| {
                let res = HttpResponse::Ok().json(&t);
                match r {
                    Ok(opt) => actix::spawn(
                        cache
                            .if_replace_cache(opt)
                            .add_topic(t)
                            .map_err(move |t| cache.add_fail_topic(t)),
                    ),
                    Err(_) => cache.add_fail_topic(t),
                };
                res
            })
        })
}

pub fn add_post(
    global: Data<GlobalVars>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let req = PostRequest {
        id: None,
        user_id: Some(1),
        topic_id: Some(1),
        category_id: 1,
        post_id: Some(1),
        post_content: Some("t4265335423646e".to_owned()),
        is_locked: None,
    };
    db.add_post(req, global.get_ref())
        .from_err()
        .and_then(move |p| {
            cache.check_cache_conn().then(move |r| {
                let res = HttpResponse::Ok().json(&p);
                match r {
                    Ok(opt) => actix::spawn(
                        cache
                            .if_replace_cache(opt)
                            .add_post(p)
                            .map_err(move |p| cache.add_fail_post(p)),
                    ),
                    Err(_) => cache.add_fail_post(p),
                };
                res
            })
        })
}

pub type Pool = l337::Pool<l337_postgres::PostgresConnectionManager<tokio_postgres::NoTls>>;

pub fn build_pool(sys: &mut actix_rt::SystemRunner) -> Pool {
    let config = tokio_postgres::Config::new()
        .host("localhost")
        .user("postgres")
        .password("123")
        .dbname("test")
        .clone();
    let manager = l337_postgres::PostgresConnectionManager::new(config, tokio_postgres::NoTls);
    let cfg = l337::Config {
        min_size: 1,
        max_size: 12,
    };
    sys.block_on(l337::Pool::new(manager, cfg)).unwrap()
}

pub fn pool(pool: Data<Pool>) -> impl Future<Item = HttpResponse, Error = Error> {
    test_pool(pool.get_ref())
        .from_err()
        .map(|t| HttpResponse::Ok().json(&t))
}

pub fn raw(db: Data<DatabaseService>) -> impl Future<Item = HttpResponse, Error = Error> {
    let ids = vec![
        1u32, 20, 11, 9, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 17, 18, 19,
    ];

    db.get_ref().check_conn().from_err().and_then(move |opt| {
        db.if_replace_db(opt)
            .get_topics_by_id_with_uid(ids)
            .from_err()
            .and_then(move |(t, ids)| {
                db.get_users_by_id(&ids)
                    .from_err()
                    .and_then(move |u| HttpResponse::Ok().json(&Topic::attach_users(&t, &u)))
            })
    })
}

pub fn raw_cache(cache: Data<CacheService>) -> impl Future<Item = HttpResponse, Error = Error> {
    cache
        .get_topics_pop(1, 1)
        .from_err()
        .and_then(move |(t, ids)| {
            cache
                .get_users_from_ids(ids)
                .from_err()
                .and_then(move |u| HttpResponse::Ok().json(&Topic::attach_users(&t, &u)))
        })
}

fn test_pool(pool: &Pool) -> impl Future<Item = Vec<Topic>, Error = ResError> {
    pool.connection()
        .map_err(|_| ResError::InternalServerError)
        .and_then(|mut c| {
            c.client
                .prepare("SELECT * FROM topics WHERE id = ANY($1)")
                .map_err(|_| ResError::InternalServerError)
                .and_then(move |stmt| {
                    c.client
                        .query(
                            &stmt,
                            &[&vec![
                                1u32, 20, 11, 9, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16,
                                17, 18, 19,
                            ]],
                        )
                        .fold(Vec::with_capacity(20), move |mut v, r| {
                            if let Ok(r) = Topic::try_from(r) {
                                v.push(r)
                            }
                            Ok::<_, _>(v)
                        })
                        .map_err(|_| ResError::InternalServerError)
                })
        })
}
