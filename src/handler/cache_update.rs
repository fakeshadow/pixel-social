use std::{future::Future, pin::Pin, time::Duration};

use chrono::Utc;
use heng_rs::{Context, Scheduler, SchedulerSender};
use redis::{aio::SharedConnection, cmd, pipe};

use crate::handler::{
    cache::{MyRedisPool, POOL_REDIS},
    db::POOL,
    messenger::ErrRepTaskAddr,
};
use crate::model::{cache_schema::HashMapBrown, common::dur, errors::ResError};

const LIST_INTERVAL: Duration = dur(5000);
const FAILED_INTERVAL: Duration = dur(3000);

struct RedisFailedTask {
    rep_addr: Option<ErrRepTaskAddr>,
}

impl RedisFailedTask {
    async fn update_failed(&mut self, msg: &CacheFailedMessage) -> Result<(), ResError> {
        match *msg {
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

impl Scheduler for RedisFailedTask {
    type Message = CacheFailedMessage;

    fn handler<'a>(
        &'a mut self,
        ctx: &'a mut Context<Self>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            while let Some(msg) = ctx.get_msg_front() {
                if let Err(e)  = self.update_failed(&msg).await {
                    ctx.push_msg_front(msg);
                    if let Some(addr) = self.rep_addr.as_ref() {
                        let _ = addr.send(e).await;
                    }
                    return;
                }
            }
        })
    }
}

struct RedisListTask {
    rep_addr: Option<ErrRepTaskAddr>,
}

impl Scheduler for RedisListTask {
    type Message = ();

    fn handler<'a>(
        &'a mut self,
        _ctx: &'a mut Context<Self>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            if let Err(e) = POOL_REDIS.handle_list_update().await {
                if let Some(addr) = self.rep_addr.as_ref() {
                    let _ = addr.send(e).await;
                }
            }
        })
    }
}

pub(crate) type RedisFailedTaskSender = SchedulerSender<CacheFailedMessage>;

// We have to return all the addresses.
// Because if a address goes out of the scope the tasks's context will lose it's ability to access the Signal receiver and cause an error.
pub(crate) fn init_cache_update_services(
    rep_addr: Option<ErrRepTaskAddr>,
) -> (RedisFailedTaskSender, SchedulerSender<()>) {
    let list_task = RedisListTask {
        rep_addr: rep_addr.clone(),
    };
    let addr_temp = list_task.start_with_handler(LIST_INTERVAL);

    let failed_task = RedisFailedTask {
        rep_addr
    };
    let addr = failed_task.start_with_handler(FAILED_INTERVAL);

    (addr, addr_temp)
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
