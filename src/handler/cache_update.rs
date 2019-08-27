use std::cell::RefCell;
use std::sync::Mutex;
use std::time::Duration;

use actix::{Actor, ActorFuture, Addr, AsyncContext, Context, Handler, Message, WrapFuture};
use chrono::Utc;
use futures01::{Future as Future01, future::Either};
use redis::{aio::SharedConnection, cmd, pipe};

use crate::handler::cache::{
    AddToCache, CategoriesFromCache, CheckCacheConn, FromCache, GetSharedConn, IdsFromList,
};
use crate::model::{
    cache_schema::HashMapBrown,
    cache_update::{FailedCollection, FailedType},
    category::Category,
    errors::ResError,
    post::Post,
    topic::Topic,
    user::User,
};

// list_pop update interval time gap in seconds
const LIST_TIME_DUR: Duration = Duration::from_secs(5);
// time interval for retry adding failed cache to redis.
const FAILED_TIME_DUR: Duration = Duration::from_secs(3);

// actor the same as CacheService except it runs interval functions on start up.
pub struct CacheUpdateService {
    pub url: String,
    pub cache: RefCell<SharedConnection>,
    pub failed_collection: Mutex<FailedCollection>,
}

impl Actor for CacheUpdateService {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.start_interval(ctx);
    }
}

impl CacheUpdateService {
    pub(crate) async fn init(redis_url: &str) -> Result<Addr<CacheUpdateService>, ResError> {
        let cache = crate::handler::cache::connect_cache(redis_url)
            .await?
            .ok_or(ResError::RedisConnection)?;

        let url = redis_url.to_owned();

        Ok(CacheUpdateService::create(move |_| CacheUpdateService {
            url,
            cache: RefCell::new(cache),
            failed_collection: Mutex::new(FailedCollection::default()),
        }))
    }

    pub(crate) fn start_interval(&mut self, ctx: &mut Context<Self>) {
        self.update_list_pop(ctx);
        self.update_failed_cache(ctx);
    }

    // use only this interval to reconnect redis if the connection is lost.
    fn update_list_pop(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(LIST_TIME_DUR, move |act, ctx| {
            ctx.spawn(
                act.check_cache_conn_01()
                    .into_actor(act)
                    .and_then(|opt, act, _| {
                        act.if_replace_cache(opt)
                            .categories_from_cache_01()
                            .into_actor(act)
                            .and_then(|cat, act, _| {
                                let conn = act.get_conn();
                                let yesterday =
                                    Utc::now().naive_utc().timestamp_millis() - 86_400_000;
                                let mut vec = Vec::new();

                                for c in cat.iter() {
                                    // update_list will also update topic count new.
                                    vec.push(Either::A(update_list_01(
                                        Some(c.id),
                                        yesterday,
                                        conn.clone(),
                                    )));
                                    vec.push(Either::B(update_post_count_01(
                                        c.id,
                                        yesterday,
                                        conn.clone(),
                                    )));
                                }
                                vec.push(Either::A(update_list_01(None, yesterday, conn)));

                                futures01::future::join_all(vec).map(|_| ()).into_actor(act)
                            })
                    })
                    .map_err(|_e: ResError, _, _| ()),
            );
        });
    }

    // ToDo: right now every failed cache is update individually. Could use a giant pipeline to reduce some traffic if there are major lost connection to redis occur often.
    fn update_failed_cache(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(FAILED_TIME_DUR, move |act, ctx| {
            let mut v = Vec::new();

            let mut u_t = Vec::new();
            let mut u_p = Vec::new();
            let mut u_u = Vec::new();
            let mut u_c = Vec::new();

            let mut uids = Vec::new();
            let mut pids = Vec::new();
            let mut tids = Vec::new();
            let mut cids = Vec::new();

            if let Ok(l) = act.failed_collection.lock() {
                for (c, typ) in l.category.iter() {
                    cids.push(c.id);
                    match *typ {
                        FailedType::New => v.push(act.add_category_cache_01(c)),
                        FailedType::Update => u_c.push(c), //ToDo: add update category cache.
                    };
                }

                for (t, typ) in l.topic.iter() {
                    tids.push(t.id);
                    match *typ {
                        FailedType::New => v.push(act.add_topic_cache_01(t)),
                        FailedType::Update => u_t.push(t),
                    };
                }

                for (p, typ) in l.post.iter() {
                    pids.push(p.id);
                    match *typ {
                        FailedType::New => v.push(act.add_post_cache_01(&p)),
                        FailedType::Update => u_p.push(p),
                    };
                }

                for (u, _) in l.user.iter() {
                    uids.push(u.id);
                    u_u.push(u)
                }

                if !u_t.is_empty() || !u_p.is_empty() || !u_u.is_empty() || !u_c.is_empty() {
                    v.push(act.bulk_add_update_cache_01(u_t, u_p, u_u, u_c));
                }
            };

            if !v.is_empty() {
                ctx.spawn(
                    futures01::future::join_all(v)
                        .map_err(|_| ())
                        .into_actor(act)
                        .and_then(move |_, act, _| {
                            if let Ok(mut l) = act.failed_collection.lock() {
                                l.remove_by_pids(&pids);
                                l.remove_by_tids(&tids);
                                l.remove_by_uids(&uids);
                                l.remove_by_cids(&cids);
                            }
                            actix::fut::ok(())
                        }),
                );
            }
        });
    }
}

type ListWithSortedRange = (HashMapBrown<u32, i64>, Vec<(u32, u32)>);

fn update_list_01(
    cid: Option<u32>,
    yesterday: i64,
    conn: SharedConnection,
) -> impl Future01<Item=(), Error=ResError> {
    let (list_key, time_key, reply_key, set_key) = match cid.as_ref() {
        Some(cid) => (
            format!("category:{}:list_pop", cid),
            format!("category:{}:topics_time", cid),
            format!("category:{}:topics_reply", cid),
            Some(format!("category:{}:set", cid)),
        ),
        None => (
            "category:all:list_pop".to_owned(),
            "category:all:topics_time".to_owned(),
            "category:all:topics_reply".to_owned(),
            None,
        ),
    };

    let mut pip = pipe();
    pip.atomic();
    pip.cmd("ZREVRANGEBYSCORE")
        .arg(time_key.as_str())
        .arg("+inf")
        .arg(yesterday)
        .arg("WITHSCORES")
        .cmd("ZREVRANGEBYSCORE")
        .arg(reply_key.as_str())
        .arg("+inf")
        .arg("-inf")
        .arg("WITHSCORES");

    pip.query_async(conn).map_err(ResError::from).and_then(
        move |(conn, (HashMapBrown(tids), counts)): (_, ListWithSortedRange)| {
            use std::cmp::Ordering;

            let mut counts = counts
                .into_iter()
                .filter(|(tid, _)| tids.contains_key(tid))
                .collect::<Vec<(u32, u32)>>();

            counts.sort_by(|(a0, a1), (b0, b1)| {
                if a1 == b1 {
                    if let Some(a) = tids.get(a0) {
                        if let Some(b) = tids.get(b0) {
                            if a > b {
                                return Ordering::Less;
                            } else if a < b {
                                return Ordering::Greater;
                            };
                        }
                    }
                    Ordering::Equal
                } else {
                    Ordering::Greater
                }
            });

            let counts = counts.into_iter().map(|(id, _)| id).collect::<Vec<u32>>();

            let mut should_update = false;
            let mut pip = pipe();
            pip.atomic();

            if !tids.is_empty() {
                if let Some(key) = set_key {
                    pip.cmd("HSET")
                        .arg(&key)
                        .arg("topic_count_new")
                        .arg(tids.len())
                        .ignore();
                    should_update = true;
                }
            }

            if !counts.is_empty() {
                pip.cmd("del")
                    .arg(&list_key)
                    .ignore()
                    .cmd("rpush")
                    .arg(&list_key)
                    .arg(counts)
                    .ignore();
                should_update = true;
            }

            if should_update {
                Either::A(
                    pip.query_async(conn)
                        .map_err(ResError::from)
                        .map(|(_, ())| ()),
                )
            } else {
                Either::B(futures01::future::ok(()))
            }
        },
    )
}

fn update_post_count_01(
    cid: u32,
    yesterday: i64,
    conn: SharedConnection,
) -> impl Future01<Item=(), Error=ResError> {
    let time_key = format!("category:{}:posts_time", cid);
    let set_key = format!("category:{}:set", cid);

    cmd("ZCOUNT")
        .arg(time_key.as_str())
        .arg(yesterday)
        .arg("+inf")
        .query_async(conn)
        .map_err(ResError::from)
        .and_then(move |(conn, count): (_, u32)| {
            if count > 0 {
                Either::A(
                    cmd("HMSET")
                        .arg(set_key.as_str())
                        .arg(&[("post_count_new", count)])
                        .query_async(conn)
                        .map_err(ResError::from)
                        .map(|(_, ())| ()),
                )
            } else {
                Either::B(futures01::future::ok(()))
            }
        })
}

impl CheckCacheConn for CacheUpdateService {
    fn self_url(&self) -> String {
        self.url.to_owned()
    }

    fn replace_cache(&self, c: SharedConnection) {
        self.cache.replace(c);
    }
}

impl GetSharedConn for CacheUpdateService {
    fn get_conn(&self) -> SharedConnection {
        self.cache.borrow().clone()
    }
}

impl IdsFromList for CacheUpdateService {}

impl FromCache for CacheUpdateService {}

impl CategoriesFromCache for CacheUpdateService {}

impl AddToCache for CacheUpdateService {}

// CacheService will push data failed to insert into redis to CacheUpdateService actor.
// we will just keep retrying to add them to redis.
#[derive(Message)]
pub enum CacheFailedMessage {
    FailedTopic(Topic),
    FailedPost(Post),
    FailedCategory(Category),
    FailedUser(Vec<User>),
    FailedTopicUpdate(Vec<Topic>),
    FailedPostUpdate(Vec<Post>),
}

impl Handler<CacheFailedMessage> for CacheUpdateService {
    type Result = ();
    fn handle(&mut self, msg: CacheFailedMessage, _: &mut Context<Self>) {
        match msg {
            CacheFailedMessage::FailedPost(p) => {
                if let Ok(mut l) = self.failed_collection.lock() {
                    l.add_post_new(p);
                }
            }
            CacheFailedMessage::FailedTopic(t) => {
                if let Ok(mut l) = self.failed_collection.lock() {
                    l.add_topic_new(t);
                }
            }
            CacheFailedMessage::FailedCategory(c) => {
                if let Ok(mut l) = self.failed_collection.lock() {
                    l.add_category_new(c);
                }
            }
            CacheFailedMessage::FailedTopicUpdate(t) => {
                if let Ok(mut l) = self.failed_collection.lock() {
                    l.add_topic_update(t);
                }
            }
            CacheFailedMessage::FailedPostUpdate(p) => {
                if let Ok(mut l) = self.failed_collection.lock() {
                    l.add_post_update(p);
                }
            }
            CacheFailedMessage::FailedUser(t) => {
                if let Ok(mut l) = self.failed_collection.lock() {
                    l.add_user(t);
                }
            }
        }
    }
}
