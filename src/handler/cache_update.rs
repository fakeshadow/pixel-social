use std::{collections::VecDeque, time::Duration};

use actix::prelude::{
    Actor, ActorFuture, Addr, AsyncContext, Context, Handler, Message, WrapFuture,
};
use chrono::Utc;
use redis::{aio::MultiplexedConnection, cmd, pipe};

use crate::handler::{
    cache::{MyRedisPool, POOL_REDIS},
    db::POOL,
    messenger::ErrorReportServiceAddr,
};
use crate::model::{cache_schema::HashMapBrown, common::dur, errors::ResError};

const LIST_INTERVAL: Duration = dur(5000);
const FAILED_INTERVAL: Duration = dur(3000);

pub struct CacheService {
    rep_addr: Option<ErrorReportServiceAddr>,
    message: VecDeque<CacheFailedMessage>,
}

impl CacheService {
    pub fn new(rep_addr: Option<ErrorReportServiceAddr>) -> Self {
        CacheService {
            rep_addr,
            message: Default::default(),
        }
    }

    async fn update_failed(msg: CacheFailedMessage) -> Result<(), ResError> {
        match msg {
            CacheFailedMessage::FailedTopic(id) => {
                let (t, _) = POOL.get_topics(&[id]).await?;
                POOL_REDIS.add_topic(&t).await
            }
            CacheFailedMessage::FailedPost(id) => {
                let (p, _) = POOL.get_posts(&[id]).await?;
                POOL_REDIS.add_post(&p).await
            }
            CacheFailedMessage::FailedCategory(id) => {
                let c = POOL.get_categories(&[id]).await?;
                POOL_REDIS.add_category(&c).await
            }
            CacheFailedMessage::FailedUser(id) => {
                let u = POOL.get_users(&[id]).await?;
                POOL_REDIS.update_users(&u).await
            }
            CacheFailedMessage::FailedTopicUpdate(id) => {
                let (t, _) = POOL.get_topics(&[id]).await?;
                POOL_REDIS.update_topics(&t).await
            }
            CacheFailedMessage::FailedPostUpdate(id) => {
                let (p, _) = POOL.get_posts(&[id]).await?;
                POOL_REDIS.update_posts(&p).await
            }
        }
    }
}

pub type CacheServiceAddr = Addr<CacheService>;

impl Actor for CacheService {
    type Context = Context<Self>;

    // when actor starts we run a interval function every 60 seconds to inject all failed cache to redis.
    // we run another interval function every 10 seconds to update redis list to correct the cache order.
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(FAILED_INTERVAL, |act, ctx| {
            while let Some(msg) = act.message.pop_front() {
                ctx.spawn(
                    Self::update_failed(msg.clone())
                        .into_actor(act)
                        .map(spawn_report),
                );
            }
        });

        ctx.run_interval(LIST_INTERVAL, |act, ctx| {
            ctx.spawn(
                POOL_REDIS
                    .handle_list_update()
                    .into_actor(act)
                    .map(spawn_report),
            );
        });
    }
}

// no type infer needed as we don't use context in the function
fn spawn_report<A>(r: Result<(), ResError>, act: &mut CacheService, _ctx: &mut A) {
    if let Err(e) = r {
        if let Some(addr) = act.rep_addr.as_ref() {
            let addr = addr.clone();
            actix_rt::spawn(async move {
                let _ = addr.send(e).await;
            })
        }
    };
}

// CacheService will push data failed to insert into redis to CacheUpdateService actor.
// we will just keep retrying to add them to redis.
#[derive(Clone, Message)]
#[rtype(result = "()")]
pub enum CacheFailedMessage {
    FailedTopic(u32),
    FailedPost(u32),
    FailedCategory(u32),
    FailedUser(u32),
    FailedTopicUpdate(u32),
    FailedPostUpdate(u32),
}

impl Handler<CacheFailedMessage> for CacheService {
    type Result = ();

    fn handle(&mut self, msg: CacheFailedMessage, _ctx: &mut Context<Self>) -> Self::Result {
        self.message.push_back(msg);
    }
}

impl MyRedisPool {
    // iterate all categories cache and update list as well as the topic/post count for every category
    async fn handle_list_update(&self) -> Result<(), ResError> {
        let cat = self.get_categories_all().await?;

        let yesterday = Utc::now().naive_utc().timestamp_millis() - 86_400_000;

        let mut pool = self.get().await?;
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
    conn: &mut MultiplexedConnection,
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
                    match a.cmp(b) {
                        Ordering::Greater => return Ordering::Less,
                        Ordering::Less => return Ordering::Greater,
                        _ => ()
                    }
                    // if a > b {
                    //     return Ordering::Less;
                    // } else if a < b {
                    //     return Ordering::Greater;
                    // };
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
    conn: &mut MultiplexedConnection,
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
