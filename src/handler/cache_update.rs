use std::collections::VecDeque;
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use chrono::Utc;
use futures::{
    channel::mpsc::UnboundedSender, lock::Mutex, FutureExt, SinkExt, StreamExt, TryFutureExt,
};
use redis::{aio::SharedConnection, cmd, pipe};

use crate::handler::db::MyPostgresPool;
use crate::handler::{cache::MyRedisPool, messenger::RepErrorAddr};
use crate::model::runtime::{SendRepError, SpawnIntervalHandlerActixRt};
use crate::model::{cache_schema::HashMapBrown, common::dur, errors::ResError};

const LIST_INTERVAL: Duration = dur(5000);
const FAILED_INTERVAL: Duration = dur(3000);

pub struct CacheUpdateService {
    pg_pool: MyPostgresPool,
    rd_pool: MyRedisPool,
    queue: Arc<Mutex<VecDeque<CacheFailedMessage>>>,
    rep_addr: Option<RepErrorAddr>,
}

impl SpawnIntervalHandlerActixRt for CacheUpdateService {
    fn handle<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = Result<(), ResError>> + Send + 'a>> {
        // ToDo: bulk operation.
        Box::pin(async move {
            let mut queue = self.queue.lock().await;
            while let Some(msg) = queue.pop_front() {
                let r = match msg {
                    CacheFailedMessage::FailedTopic(id) => {
                        let (t, _) = self.pg_pool.get_topics(&[id]).await?;
                        self.rd_pool.add_topic(&t).await
                    }
                    CacheFailedMessage::FailedPost(id) => {
                        let (p, _) = self.pg_pool.get_posts(&[id]).await?;
                        self.rd_pool.add_post(&p).await
                    }
                    CacheFailedMessage::FailedCategory(id) => {
                        let c = self.pg_pool.get_categories(&[id]).await?;
                        self.rd_pool.add_category(&c).await
                    }
                    CacheFailedMessage::FailedUser(id) => {
                        let u = self.pg_pool.get_users(&[id]).await?;
                        self.rd_pool.update_users(&u).await
                    }
                    CacheFailedMessage::FailedTopicUpdate(id) => {
                        let (t, _) = self.pg_pool.get_topics(&[id]).await?;
                        self.rd_pool.update_topics(&t).await
                    }
                    CacheFailedMessage::FailedPostUpdate(id) => {
                        let (p, _) = self.pg_pool.get_posts(&[id]).await?;
                        self.rd_pool.update_posts(&p).await
                    }
                };
                if r.is_err() {
                    queue.push_back(msg);
                    return r;
                }
            }
            Ok(())
        })
    }
}

impl SendRepError for CacheUpdateService {
    fn send_err_rep<'a>(
        &'a mut self,
        e: ResError,
    ) -> Pin<Box<dyn Future<Output = Result<(), ResError>> + Send + 'a>> {
        Box::pin(async move {
            if let Some(addr) = self.rep_addr.as_ref() {
                addr.do_send(e.into());
            }
            Ok(())
        })
    }
}

// actix::web::Data().into_inner will return our CacheUpdateAddr in an Arc.
pub type SharedCacheUpdateAddr = Arc<CacheUpdateAddr>;

// we don't need Arc wrapper here as the actix::web::Data::new() will provide the Arc layer.
/// cache update addr is used to collect failed insertion to redis. and retry them in CacheUpdateService.
pub struct CacheUpdateAddr(Mutex<UnboundedSender<CacheFailedMessage>>);

impl CacheUpdateAddr {
    pub(crate) async fn do_send(&self, msg: CacheFailedMessage) {
        let mut tx = self.0.lock().await;
        // ToDo: we should store this failure as log.
        let _ = tx.send(msg).await;
    }
}

impl CacheUpdateService {
    pub(crate) fn init(
        pg_pool: MyPostgresPool,
        rd_pool: MyRedisPool,
        rep_addr: Option<RepErrorAddr>,
    ) -> Result<CacheUpdateAddr, ResError> {
        let queue = Arc::new(Mutex::new(VecDeque::new()));

        let (tx, mut rx) = futures::channel::mpsc::unbounded::<CacheFailedMessage>();

        let queue_rx = queue.clone();
        actix::spawn(
            Box::pin(async move {
                while let Some(msg) = rx.next().await {
                    let mut queue = queue_rx.lock().await;
                    queue.push_back(msg);
                }
            })
            .unit_error()
            .compat(),
        );

        let this = CacheUpdateService {
            pg_pool,
            rd_pool: rd_pool.clone(),
            queue,
            rep_addr,
        };

        this.spawn_interval(FAILED_INTERVAL, FAILED_INTERVAL);
        // run scheduled redis lists updates. this interval is directly called onto MyRedisPool.
        rd_pool.spawn_interval(LIST_INTERVAL, LIST_INTERVAL);

        let addr = CacheUpdateAddr(Mutex::new(tx));
        Ok(addr)
    }
}

impl SpawnIntervalHandlerActixRt for MyRedisPool {
    fn handle<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = Result<(), ResError>> + Send + 'a>> {
        Box::pin(async move { self.handle_list_update().await })
    }
}

impl SendRepError for MyRedisPool {
    fn send_err_rep<'a>(
        &'a mut self,
        _e: ResError,
    ) -> Pin<Box<dyn Future<Output = Result<(), ResError>> + Send + 'a>> {
        Box::pin(async move { Ok(()) })
    }
}

impl MyRedisPool {
    // iterate all categories cache and update list as well as the topic/post count for every category
    async fn handle_list_update(&self) -> Result<(), ResError> {
        let cat = self.get_categories_all().await?;

        let yesterday = Utc::now().naive_utc().timestamp_millis() - 86_400_000;

        let mut pool = self.get_pool().await?;
        let conn = &mut *pool;

        for c in cat.iter() {
            // update_list will also update topic count new.
            let _ = update_list(Some(c.id), yesterday, conn).await;
            let _ = update_post_count(c.id, yesterday, conn).await;
        }
        let _ = update_list(None, yesterday, conn).await;

        Ok(())
    }
}

type ListWithSortedRange = (HashMapBrown<u32, i64>, Vec<(u32, u32)>);

async fn update_list(
    cid: Option<u32>,
    yesterday: i64,
    conn: &mut SharedConnection,
) -> Result<(), ResError> {
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

    let (HashMapBrown(tids), counts) = pip.query_async::<_, ListWithSortedRange>(conn).await?;

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
        pip.query_async::<_, ()>(conn).await?;
    };
    Ok(())
}

async fn update_post_count(
    cid: u32,
    yesterday: i64,
    conn: &mut SharedConnection,
) -> Result<(), ResError> {
    let time_key = format!("category:{}:posts_time", cid);
    let set_key = format!("category:{}:set", cid);

    let count = cmd("ZCOUNT")
        .arg(time_key.as_str())
        .arg(yesterday)
        .arg("+inf")
        .query_async::<_, u32>(conn)
        .await?;

    if count > 0 {
        cmd("HMSET")
            .arg(set_key.as_str())
            .arg(&[("post_count_new", count)])
            .query_async::<_, ()>(conn)
            .await?;
        Ok(())
    } else {
        Ok(())
    }
}

// CacheService will push data failed to insert into redis to CacheUpdateService actor.
// we will just keep retrying to add them to redis.
pub enum CacheFailedMessage {
    FailedTopic(u32),
    FailedPost(u32),
    FailedCategory(u32),
    FailedUser(u32),
    FailedTopicUpdate(u32),
    FailedPostUpdate(u32),
}
