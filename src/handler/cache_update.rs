use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use chrono::Utc;
use futures::{
    channel::mpsc::UnboundedReceiver, future::Either, lock::Mutex as FutMutex, StreamExt,
};
use redis::{aio::SharedConnection, cmd, pipe};
use tokio::{future::FutureExt as TokioFutureExt, timer::Interval};

use crate::handler::cache::{
    AddToCache, CategoriesFromCache, CheckRedisMut, FromCache, GetSharedConn, IdsFromList,
};
use crate::model::channel::{ChannelAddress, ChannelGenerator, InjectQueue};
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

pub struct CacheUpdateService {
    pub url: String,
    pub cache: SharedConnection,
    pub failed_collection: Arc<FutMutex<FailedCollection>>,
}

impl ChannelGenerator for CacheUpdateService {
    type Message = CacheFailedMessage;
}

impl CacheUpdateService {
    pub(crate) async fn init(
        redis_url: &str,
    ) -> Result<ChannelAddress<CacheFailedMessage>, ResError> {
        let url = redis_url.to_owned();

        let cache = crate::handler::cache::connect_cache(redis_url)
            .await?
            .ok_or(ResError::RedisConnection)?;

        // use an unbounded channel to inject msg to collection from other threads.
        let (addr, receiver) = CacheUpdateService::create_channel();

        // collection is passed to both PSNService and QueueInjector.
        let failed_collection = Arc::new(FutMutex::new(FailedCollection::default()));

        // run failed collection injector in a separate future.
        FailedCacheQueue::new(failed_collection.clone(), receiver).handle_inject();

        // User double layer of Arc<Mutex<_>> as we share failed_collection in different spawned futures.
        let update = Arc::new(FutMutex::new(CacheUpdateService {
            url,
            cache,
            failed_collection: failed_collection.clone(),
        }));

        // run interval futures which handle cache list update in a separate future.
        let upt_list = update.clone();
        tokio::spawn(async move {
            let mut interval = Interval::new_interval(LIST_TIME_DUR);
            loop {
                interval.next().await;
                // set a timeout for the looped future
                // ToDo: relax the timeout if the duration is too tight.
                let mut upt = upt_list.lock().await;
                if let Err(e) = upt.handle_list_update().timeout(LIST_TIME_DUR).await {
                    // ToDo: handler error.
                    println!("{:?}", e.to_string());
                }
            }
        });

        // run interval futures which handle failed cache update in a separate future.
        let upt_failed = update.clone();

        tokio::spawn(async move {
            let mut interval = Interval::new_interval(FAILED_TIME_DUR);
            loop {
                interval.next().await;
                let mut upt = upt_failed.lock().await;
                if let Err(e) = upt.handle_failed_update().timeout(FAILED_TIME_DUR).await {
                    // ToDo: handler error.
                    println!("{:?}", e.to_string());
                }
            }
        });

        // wrap the channel sender in Arc<Mutex> as it has to be passed to CacheService.
        Ok(addr)
    }

    // iterate all categories cache and update list as well as the topic/post count for every category
    async fn handle_list_update(&mut self) -> Result<(), ResError> {
        let cat = self
            .check_redis_mut()
            .await?
            .categories_from_cache()
            .await?;

        let conn = self.get_conn();
        let yesterday = Utc::now().naive_utc().timestamp_millis() - 86_400_000;

        let mut vec = futures::stream::FuturesUnordered::new();

        for c in cat.iter() {
            // update_list will also update topic count new.
            vec.push(Either::Right(update_list(
                Some(c.id),
                yesterday,
                conn.clone(),
            )));
            vec.push(Either::Left(update_post_count(
                c.id,
                yesterday,
                conn.clone(),
            )));
        }
        vec.push(Either::Right(update_list(None, yesterday, conn)));

        while let Some(_t) = vec.next().await {
            // ToDo: handle error
        }

        Ok(())
    }

    // iterate all categories cache and update list as well as the topic/post count for every category
    // ToDo: test async version performance.
    async fn handle_failed_update(&mut self) -> Result<(), ResError> {
        let mut v = futures::stream::FuturesUnordered::new();

        let mut u_t = Vec::new();
        let mut u_p = Vec::new();
        let mut u_u = Vec::new();
        let mut u_c = Vec::new();
        let mut uids = Vec::new();
        let mut pids = Vec::new();
        let mut tids = Vec::new();
        let mut cids = Vec::new();

        let mut collect: futures::lock::MutexGuard<FailedCollection> =
            self.failed_collection.lock().await;

        for (c, typ) in collect.category.iter() {
            cids.push(c.id);
            match *typ {
                FailedType::New => v.push(self.add_category_cache(c)),
                FailedType::Update => u_c.push(c), // ToDo: add update category cache.
            };
        }

        for (t, typ) in collect.topic.iter() {
            tids.push(t.id);
            match *typ {
                FailedType::New => v.push(self.add_topic_cache(t)),
                FailedType::Update => u_t.push(t),
            };
        }

        for (p, typ) in collect.post.iter() {
            pids.push(p.id);
            match *typ {
                FailedType::New => v.push(self.add_post_cache(&p)),
                FailedType::Update => u_p.push(p),
            };
        }

        for (u, _) in collect.user.iter() {
            uids.push(u.id);
            u_u.push(u)
        }

        if !u_t.is_empty() || !u_p.is_empty() || !u_u.is_empty() || !u_c.is_empty() {
            v.push(self.bulk_add_update_cache(u_t, u_p, u_u, u_c));
        }

        if !v.is_empty() {
            // ToDo: don't use join all as we could have some good insertion while others are failed

            while let Some(_) = v.next().await {
                // ToDo: collect all successful insertion and compare to input.
                ()
            }

            collect.remove_by_pids(&pids);
            collect.remove_by_tids(&tids);
            collect.remove_by_uids(&uids);
            collect.remove_by_cids(&cids);
        }

        Ok(())
    }
}

impl InjectQueue<CacheFailedMessage> for FailedCacheQueue {
    type Error = ResError;

    fn receiver(&mut self) -> &mut UnboundedReceiver<CacheFailedMessage> {
        &mut self.receiver
    }

    fn handle_message<'a>(
        &'a mut self,
        msg: CacheFailedMessage,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>> {
        Box::pin(async move {
            let mut collection = self.failed_collection.lock().await;
            match msg {
                CacheFailedMessage::FailedPost(p) => collection.add_post_new(p),
                CacheFailedMessage::FailedTopic(t) => collection.add_topic_new(t),
                CacheFailedMessage::FailedCategory(c) => collection.add_category_new(c),
                CacheFailedMessage::FailedTopicUpdate(t) => collection.add_topic_update(t),
                CacheFailedMessage::FailedPostUpdate(p) => collection.add_post_update(p),
                CacheFailedMessage::FailedUser(t) => collection.add_user(t),
            };
            Ok(())
        })
    }
}

struct FailedCacheQueue {
    failed_collection: Arc<FutMutex<FailedCollection>>,
    receiver: UnboundedReceiver<CacheFailedMessage>,
}

impl FailedCacheQueue {
    fn new(
        failed_collection: Arc<FutMutex<FailedCollection>>,
        receiver: UnboundedReceiver<CacheFailedMessage>,
    ) -> Self {
        FailedCacheQueue {
            failed_collection,
            receiver,
        }
    }
}

type ListWithSortedRange = (HashMapBrown<u32, i64>, Vec<(u32, u32)>);

async fn update_list(
    cid: Option<u32>,
    yesterday: i64,
    conn: SharedConnection,
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

    let (conn, (HashMapBrown(tids), counts)): (SharedConnection, ListWithSortedRange) =
        pip.query_async(conn).await?;

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
        let (_, ()) = pip.query_async(conn).await?;
    };
    Ok(())
}

async fn update_post_count(
    cid: u32,
    yesterday: i64,
    conn: SharedConnection,
) -> Result<(), ResError> {
    let time_key = format!("category:{}:posts_time", cid);
    let set_key = format!("category:{}:set", cid);

    let (conn, count): (SharedConnection, u32) = cmd("ZCOUNT")
        .arg(time_key.as_str())
        .arg(yesterday)
        .arg("+inf")
        .query_async(conn)
        .await?;

    if count > 0 {
        let (_, ()) = cmd("HMSET")
            .arg(set_key.as_str())
            .arg(&[("post_count_new", count)])
            .query_async(conn)
            .await?;
        Ok(())
    } else {
        Ok(())
    }
}

impl CheckRedisMut for CacheUpdateService {
    fn self_url(&self) -> &str {
        &self.url
    }

    fn replace_redis_mut(&mut self, c: SharedConnection) {
        self.cache = c;
    }
}

impl GetSharedConn for CacheUpdateService {
    fn get_conn(&self) -> SharedConnection {
        self.cache.clone()
    }
}

impl IdsFromList for CacheUpdateService {}

impl FromCache for CacheUpdateService {}

impl CategoriesFromCache for CacheUpdateService {}

impl AddToCache for CacheUpdateService {}

// CacheService will push data failed to insert into redis to CacheUpdateService actor.
// we will just keep retrying to add them to redis.
pub enum CacheFailedMessage {
    FailedTopic(Topic),
    FailedPost(Post),
    FailedCategory(Category),
    FailedUser(Vec<User>),
    FailedTopicUpdate(Vec<Topic>),
    FailedPostUpdate(Vec<Post>),
}
