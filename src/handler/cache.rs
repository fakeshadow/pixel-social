use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use chrono::NaiveDateTime;
use futures::TryFutureExt;
use once_cell::sync::OnceCell;
use redis::{aio::MultiplexedConnection, cmd, pipe, Pipeline};
use redis_tang::{Builder, Pool, PoolRef, RedisManager};

use crate::model::{
    cache_schema::{HashMapBrown, RefTo},
    category::Category,
    common::{SelfId, SelfIdString, SelfUserId},
    errors::ResError,
    post::Post,
    topic::Topic,
    user::User,
};

// page offsets of list query
const LIMIT: usize = 20;
// use LEX_BASE minus pid and tid before adding to zrange.
const LEX_BASE: u32 = std::u32::MAX;

// hash life is expire time of topic and post hash in seconds.
const HASH_LIFE: usize = 172_800;
// mail life is expire time of mail hash in seconds
const MAIL_LIFE: usize = 3600;

pub const CATEGORY_U8: &[u8] = b"category:";
pub const TOPIC_U8: &[u8] = b"topic:";
pub const USER_U8: &[u8] = b"user:";
pub const POST_U8: &[u8] = b"post:";
pub const USER_PSN_U8: &[u8] = b"user_psn:";
const SET_U8: &[u8] = b":set";
const PERM_U8: &[u8] = b"_perm";

pub fn pool_redis() -> &'static MyRedisPool {
    static POOL_REDIS: OnceCell<MyRedisPool> = OnceCell::new();

    POOL_REDIS.get_or_init(|| {
        MyRedisPool::new(
            std::env::var("REDIS_URL")
                .expect("REDIS_URL must be set in .env")
                .as_str(),
        )
    })
}

#[derive(Clone)]
pub struct MyRedisPool(Pool<RedisManager>);

pub type MyRedisPoolRef<'a> = PoolRef<'a, RedisManager>;

impl MyRedisPool {
    pub(crate) fn new(redis_url: &str) -> MyRedisPool {
        let mgr = RedisManager::new(redis_url);

        let pool = Builder::new()
            .always_check(false)
            .idle_timeout(None)
            .max_lifetime(None)
            .min_idle(1)
            .max_size(12)
            .build_uninitialized(mgr);

        MyRedisPool(pool)
    }

    pub(crate) async fn init(&self) {
        self.0
            .init()
            .await
            .expect("Failed to initialize redis pool");
    }

    pub(crate) fn get(&self) -> impl Future<Output = Result<MyRedisPoolRef, ResError>> {
        self.0.get().err_into()
    }
}

impl MyRedisPool {
    pub(crate) async fn add_activation_mail_cache(
        mut conn: MultiplexedConnection,
        uid: u32,
        uuid: String,
        mail: String,
    ) -> Result<(), ResError> {
        let count = cmd("ZCOUNT")
            .arg("mail_queue")
            .arg(uid)
            .arg(uid)
            .query_async::<_, usize>(&mut conn)
            .await?;

        if count > 0 {
            return Ok(());
        }

        let mut pip = pipe();
        pip.atomic();
        pip.cmd("ZADD")
            .arg("mail_queue")
            .arg(uid)
            .arg(mail.as_str())
            .ignore()
            .cmd("HSET")
            .arg(uuid.as_str())
            .arg("user_id")
            .arg(uid)
            .ignore()
            .cmd("EXPIRE")
            .arg(uuid.as_str())
            .arg(MAIL_LIFE)
            .ignore();

        pip.query_async(&mut conn).err_into().await
    }

    pub(crate) async fn get_hash_map_brown(
        &self,
        key: &str,
    ) -> Result<HashMapBrown<String, String>, ResError> {
        let mut conn = self.get().await?.get_conn().clone();
        cmd("HGETALL")
            .arg(key)
            .query_async(&mut conn)
            .err_into()
            .await
    }

    pub(crate) async fn get_queue(&self, key: &str) -> Result<String, ResError> {
        let mut pip = pipe();
        pip.atomic();
        pip.cmd("zrange")
            .arg(key)
            .arg(0)
            .arg(0)
            .cmd("ZREMRANGEBYRANK")
            .arg(key)
            .arg(0)
            .arg(0);

        let mut conn = self.get().await?.get_conn().clone();
        let (mut s, ()) = pip.query_async::<_, (Vec<String>, ())>(&mut conn).await?;

        s.pop().ok_or(ResError::NoCache)
    }

    pub(crate) fn del_cache(
        mut conn: MultiplexedConnection,
        key: String,
    ) -> Pin<Box<dyn Future<Output = Result<(), ResError>> + Send>> {
        Box::pin(async move {
            let conn = &mut conn;
            cmd("del")
                .arg(key.as_str())
                .query_async(conn)
                .err_into()
                .await
        })
    }
}

// methods get cache from redis
impl MyRedisPool {
    pub(crate) async fn get_cache_with_uids_from_list<T>(
        &self,
        list_key: &str,
        page: usize,
        set_key: &[u8],
    ) -> Result<(Vec<T>, Vec<u32>), ResError>
    where
        T: Send + redis::FromRedisValue + SelfUserId + 'static,
    {
        let start = (page - 1) * 20;
        let end = start + LIMIT - 1;

        let mut conn = self.get().await?.get_conn().clone();
        let ids = Self::ids_from_list(&mut conn, list_key, start, end).await?;
        Self::from_redis_with_perm_uids(&mut conn, ids, set_key).await
    }

    pub(crate) async fn get_cache_from_list<T>(
        &self,
        list_key: &str,
        set_key: &[u8],
        start: usize,
        end: usize,
        have_perm_fields: bool,
        // return input ids so the following function can also include the ids when mapping error to ResError::IdsFromCache.
    ) -> Result<Vec<T>, ResError>
    where
        T: Send + redis::FromRedisValue + 'static,
    {
        let mut conn = self.get().await?.get_conn().clone();
        let ids = Self::ids_from_list(&mut conn, list_key, start, end).await?;
        Self::from_redis(&mut conn, ids, set_key, have_perm_fields).await
    }

    pub(crate) fn get_cache_with_uids_from_zrevrange_reverse_lex<'a, 'b: 'a, T>(
        &'a self,
        zrange_key: &'b str,
        page: usize,
        set_key: &'a [u8],
    ) -> impl Future<Output = Result<(Vec<T>, Vec<u32>), ResError>> + 'a
    where
        T: Send + redis::FromRedisValue + SelfUserId + 'static,
    {
        self.cache_with_uids_from_zrange(zrange_key, page, set_key, true, true)
    }

    pub(crate) fn get_cache_with_uids_from_zrevrange<'a, T>(
        &'a self,
        zrange_key: &'a str,
        page: usize,
        set_key: &'a [u8],
    ) -> impl Future<Output = Result<(Vec<T>, Vec<u32>), ResError>> + 'a
    where
        T: Send + redis::FromRedisValue + SelfUserId + 'static,
    {
        self.cache_with_uids_from_zrange(zrange_key, page, set_key, true, false)
    }

    pub(crate) fn get_cache_with_uids_from_zrange<'a, T>(
        &'a self,
        zrange_key: &'a str,
        page: usize,
        set_key: &'a [u8],
    ) -> impl Future<Output = Result<(Vec<T>, Vec<u32>), ResError>> + 'a
    where
        T: Send + redis::FromRedisValue + SelfUserId + 'static,
    {
        self.cache_with_uids_from_zrange(zrange_key, page, set_key, false, false)
    }

    pub(crate) async fn get_cache_with_perm_with_uids<T>(
        &self,
        ids: Vec<u32>,
        set_key: &[u8],
    ) -> Result<(Vec<T>, Vec<u32>), ResError>
    where
        T: Send + redis::FromRedisValue + SelfUserId + 'static,
    {
        let mut conn = self.get().await?.get_conn().clone();
        let t = Self::from_redis::<T>(&mut conn, ids, set_key, true).await?;

        let len = t.len();
        let mut uids = Vec::with_capacity(len);
        for t in t.iter() {
            uids.push(t.get_user_id());
        }

        Ok((t, uids))
    }

    pub(crate) async fn get_cache<T>(
        &self,
        ids: Vec<u32>,
        set_key: &[u8],
        have_perm_fields: bool,
        // return input ids so the following function can also include the ids when mapping error to ResError::IdsFromCache.
    ) -> Result<Vec<T>, ResError>
    where
        T: Send + redis::FromRedisValue + 'static,
    {
        let mut conn = self.get().await?.get_conn().clone();
        Self::from_redis(&mut conn, ids, set_key, have_perm_fields).await
    }

    pub(crate) async fn get_cache_single<T>(&self, key: &str, set_key: &str) -> Result<T, ResError>
    where
        T: Send + redis::FromRedisValue + 'static,
    {
        let mut conn = self.get().await?.get_conn().clone();

        cmd("HGETALL")
            .arg(&format!("{}:{}:set", set_key, key))
            .query_async(&mut conn)
            .err_into()
            .await
    }

    async fn cache_with_uids_from_zrange<T>(
        &self,
        zrange_key: &str,
        page: usize,
        set_key: &[u8],
        is_rev: bool,
        is_reverse_lex: bool,
    ) -> Result<(Vec<T>, Vec<u32>), ResError>
    where
        T: Send + redis::FromRedisValue + SelfUserId + 'static,
    {
        let mut conn = self.get().await?.get_conn().clone();

        let mut ids = Self::ids_from_zrange(&mut conn, zrange_key, is_rev, (page - 1) * 20).await?;

        if is_reverse_lex {
            ids = ids.into_iter().map(|i| LEX_BASE - i).collect();
        }

        Self::from_redis_with_perm_uids(&mut conn, ids, set_key).await
    }

    async fn from_redis_with_perm_uids<T>(
        conn: &mut MultiplexedConnection,
        ids: Vec<u32>,
        set_key: &[u8],
    ) -> Result<(Vec<T>, Vec<u32>), ResError>
    where
        T: Send + redis::FromRedisValue + SelfUserId + 'static,
    {
        let t: Vec<T> = Self::from_redis(conn, ids, set_key, true).await?;

        let len = t.len();

        let mut uids = Vec::with_capacity(len);
        for t in t.iter() {
            uids.push(t.get_user_id());
        }

        Ok((t, uids))
    }

    pub(crate) async fn from_redis<T>(
        conn: &mut MultiplexedConnection,
        ids: Vec<u32>,
        set_key: &[u8],
        have_perm_fields: bool,
        // return input ids so the following function can also include the ids when mapping error to ResError::IdsFromCache.
    ) -> Result<Vec<T>, ResError>
    where
        T: Send + redis::FromRedisValue + 'static,
    {
        let pip = AsyncPipelineGet {
            ids: &ids,
            set_key,
            have_perm_fields,
        };

        match pip.await.query_async::<_, Vec<T>>(conn).await {
            Ok(v) => {
                if v.len() != ids.len() {
                    Err(ResError::IdsFromCache(ids))
                } else {
                    Ok(v)
                }
            }
            Err(_) => Err(ResError::IdsFromCache(ids)),
        }
    }
}

// methods add cache to redis.
impl MyRedisPool {
    pub(crate) async fn add_topic(&self, topics: &[Topic]) -> Result<(), ResError> {
        let topic = topics.first().ok_or(ResError::InternalServerError)?;

        let pip = AsyncPipelineTopic { topic };
        let pip = pip.await;

        let mut conn = self.get().await?.get_conn().clone();
        pip.query_async(&mut conn).err_into().await
    }

    pub(crate) async fn add_post(&self, posts: &[Post]) -> Result<(), ResError> {
        let post = posts.first().ok_or(ResError::InternalServerError)?;
        let pip = AsyncPipelinePost { post };
        let pip = pip.await;

        let mut conn = self.get().await?.get_conn().clone();
        pip.query_async(&mut conn).err_into().await
    }

    pub(crate) async fn add_category(&self, category: &[Category]) -> Result<(), ResError> {
        let category = category.first().ok_or(ResError::InternalServerError)?;
        let id = category.id;
        let category: Vec<(&str, Vec<u8>)> = category.ref_to();

        let mut pip = pipe();
        pip.atomic();

        pip.cmd("rpush")
            .arg("category_id:meta")
            .arg(id)
            .ignore()
            .cmd("HMSET")
            .arg(&format!("category:{}:set", id))
            .arg(category)
            .ignore();

        let mut conn = self.get().await?.get_conn().clone();
        pip.query_async(&mut conn).err_into().await
    }

    //    pub(crate) async fn bulk_add_update_cache(
    //        &self,
    //        t: Vec<&Topic>,
    //        p: Vec<&Post>,
    //        u: Vec<&User>,
    //        c: Vec<&Category>,
    //    ) -> Result<(), ResError> {
    //        //        let pip = AsyncPipelineBulk {
    //        //            posts,
    //        //            topics,
    //        //            users,
    //        //            categories,
    //        //        };
    //        //
    //        //        let pip = pip.await;
    //
    //        let mut pip = pipe();
    //        pip.atomic();
    //
    //        let mut key = Vec::with_capacity(28);
    //        key.extend_from_slice(TOPIC_U8);
    //
    //        for t in t.into_iter() {
    //            let mut key = key.clone();
    //            key.extend_from_slice(t.self_id_string().as_bytes());
    //            key.extend_from_slice(SET_U8);
    //
    //            let t: Vec<(&'static str, Vec<u8>)> = t.ref_to();
    //
    //            pip.cmd("HMSET")
    //                .arg(key.as_slice())
    //                .arg(t)
    //                .ignore()
    //                .cmd("expire")
    //                .arg(key.as_slice())
    //                .arg(HASH_LIFE)
    //                .ignore();
    //        }
    //
    //        let mut key = Vec::with_capacity(28);
    //        key.extend_from_slice(POST_U8);
    //
    //        for p in p.into_iter() {
    //            let mut key = key.clone();
    //            key.extend_from_slice(p.self_id_string().as_bytes());
    //            key.extend_from_slice(SET_U8);
    //
    //            let p: Vec<(&'static str, Vec<u8>)> = p.ref_to();
    //
    //            pip.cmd("HMSET")
    //                .arg(key.as_slice())
    //                .arg(p)
    //                .ignore()
    //                .cmd("expire")
    //                .arg(key.as_slice())
    //                .arg(HASH_LIFE)
    //                .ignore();
    //        }
    //
    //        let mut key = Vec::with_capacity(28);
    //        key.extend_from_slice(USER_U8);
    //
    //        for u in u.into_iter() {
    //            let mut key = key.clone();
    //            key.extend_from_slice(u.self_id_string().as_bytes());
    //            key.extend_from_slice(SET_U8);
    //
    //            let u: Vec<(&'static str, Vec<u8>)> = u.ref_to();
    //
    //            pip.cmd("HMSET").arg(key.as_slice()).arg(u).ignore();
    //        }
    //
    //        let mut key = Vec::with_capacity(28);
    //        key.extend_from_slice(CATEGORY_U8);
    //
    //        for c in c.into_iter() {
    //            let mut key = key.clone();
    //            key.extend_from_slice(c.self_id_string().as_bytes());
    //            key.extend_from_slice(SET_U8);
    //
    //            let c: Vec<(&'static str, Vec<u8>)> = c.ref_to();
    //
    //            pip.cmd("HMSET").arg(key.as_slice()).arg(c).ignore();
    //        }
    //
    //        let conn = self.get_pool().await?.get_conn().clone();
    //        pip.query_async::<_, ()>(conn).await?;
    //
    //        Ok(())
    //    }

    pub(crate) async fn build_sets<'a, T>(
        &self,
        slice: &'a [T],
        set_key: &'a [u8],
        should_expire: bool,
    ) -> Result<(), ResError>
    where
        T: SelfIdString + RefTo<Vec<(&'a str, Vec<u8>)>>,
    {
        let pip = AsyncPipelineBuild {
            slice,
            set_key,
            should_expire,
        };

        let pipe = pip.await;
        let mut conn = self.get().await?.get_conn().clone();

        actix_rt::spawn(async move {
            let _ = pipe.query_async::<_, ()>(&mut conn).await;
            println!("updated cache");
        });
        Ok(())
    }
}

// methods for get indexing from redis.
/// ids from list and sorted set will return an error if the result is empty.
/// we assume no data can be found on database if we don't have according id in cache.
impl MyRedisPool {
    async fn ids_from_list(
        conn: &mut MultiplexedConnection,
        list_key: &str,
        start: usize,
        end: usize,
    ) -> Result<Vec<u32>, ResError> {
        let ids = cmd("lrange")
            .arg(list_key)
            .arg(start)
            .arg(end)
            .query_async::<_, Vec<u32>>(conn)
            .await?;

        if ids.is_empty() {
            Err(ResError::NoContent)
        } else {
            Ok(ids)
        }
    }

    async fn ids_from_zrange(
        conn: &mut MultiplexedConnection,
        list_key: &str,
        is_rev: bool,
        offset: usize,
    ) -> Result<Vec<u32>, ResError> {
        let (cmd_key, start, end) = if is_rev {
            ("zrevrangebyscore", "+inf", "-inf")
        } else {
            ("zrangebyscore", "-inf", "+inf")
        };

        let ids = cmd(cmd_key)
            .arg(list_key)
            .arg(start)
            .arg(end)
            .arg("LIMIT")
            .arg(offset)
            .arg(20)
            .query_async::<_, Vec<u32>>(conn)
            .await?;

        if ids.is_empty() {
            Err(ResError::NoContent)
        } else {
            Ok(ids)
        }
    }
}

struct AsyncPipelineTopic<'a> {
    topic: &'a Topic,
}

impl Future for AsyncPipelineTopic<'_> {
    type Output = Pipeline;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        let mut pip = pipe();
        pip.atomic();

        let t = self.topic;

        let tid = t.id;
        let cid = t.category_id;
        let time = t.created_at.timestamp_millis();
        let key = format!("topic:{}:set", t.self_id());
        let t: Vec<(&str, Vec<u8>)> = t.ref_to();

        // write hash map set
        pip.cmd("HMSET")
            .arg(key.as_str())
            .arg(t)
            .ignore()
            // set expire time for above set
            .cmd("EXPIRE")
            .arg(key.as_str())
            .arg(HASH_LIFE)
            .ignore()
            // update category's topic_count
            .cmd("HINCRBY")
            .arg(&format!("category:{}:set", cid))
            .arg("topic_count")
            .arg(1)
            .ignore()
            // add self time to category's topics_time sorted set
            .cmd("ZADD")
            .arg("category:all:topics_time")
            .arg(time)
            .arg(tid)
            .ignore()
            .cmd("ZADD")
            .arg(&format!("category:{}:topics_time", cid))
            .arg(time)
            .arg(tid)
            .ignore()
            // add self reply count to category:all's topics_reply sorted set
            .cmd("ZINCRBY")
            .arg("category:all:topics_reply")
            .arg(0)
            .arg(tid)
            .ignore()
            .cmd("ZINCRBY")
            .arg(&format!("category:{}:topics_reply", cid))
            .arg(0)
            .arg(tid)
            .ignore();

        Poll::Ready(pip)
    }
}

struct AsyncPipelinePost<'a> {
    post: &'a Post,
}

impl Future for AsyncPipelinePost<'_> {
    type Output = Pipeline;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        let p = self.post;

        let cid = p.category_id;
        let tid = p.topic_id;
        let pid = p.id;
        let post_id = p.post_id;
        let time = p.created_at;
        let time_string = time.to_string();

        let mut pip = pipe();
        pip.atomic();

        let post_key = format!("post:{}:set", pid);
        let time = time.timestamp_millis();
        let p: Vec<(&str, Vec<u8>)> = p.ref_to();

        // write hash map set
        pip.cmd("HMSET")
            .arg(post_key.as_str())
            .arg(p)
            .ignore()
            // set expire time for above set
            .cmd("EXPIRE")
            .arg(post_key.as_str())
            .arg(HASH_LIFE)
            .ignore()
            // update category's post_count
            .cmd("HINCRBY")
            .arg(&format!("category:{}:set", cid))
            .arg("post_count")
            .arg(1)
            .ignore()
            // update topic's reply_count
            .cmd("HINCRBY")
            .arg(&format!("topic:{}:set_perm", tid))
            .arg("reply_count")
            .arg(1)
            .ignore()
            // update topic's last_reply_time
            .cmd("HSET")
            .arg(&format!("topic:{}:set_perm", tid))
            .arg("last_reply_time")
            .arg(&time_string)
            .ignore()
            // add self id to topic's post_reply sorted set.
            // use LEX_BASE - pid to maintain a reversed lex order for pids have the same reply score.
            // so all posts with the same reply count will present in a pid ascend order.(when using zrevrange to query)
            .cmd("ZINCRBY")
            .arg(&format!("topic:{}:posts_reply", tid))
            .arg(0)
            .arg(LEX_BASE - pid)
            .ignore()
            // add self post time to topic's post_time sorted set.
            .cmd("ZADD")
            .arg(&format!("topic:{}:posts_time_created", tid))
            .arg(time)
            .arg(pid)
            .ignore()
            // add self post time(as topic's last reply time) to category's topics_time sorted set
            // use XX as we only want to update topic's last_reply_time in the target sorted set.
            .cmd("ZADD")
            .arg(&format!("category:{}:topics_time", cid))
            .arg("XX")
            .arg(time)
            .arg(tid)
            .ignore()
            // add self post time(as topic's last reply time) to category:all's topics_time sorted set
            // use XX the same reason as above
            .cmd("ZADD")
            .arg("category:all:topics_time")
            .arg("XX")
            .arg(time)
            .arg(tid)
            .ignore()
            // update topic's topic_reply score.
            .cmd("ZINCRBY")
            .arg(&format!("category:{}:topics_reply", cid))
            .arg(1)
            .arg(tid)
            .ignore()
            // update topic's topic_reply score.
            .cmd("ZINCRBY")
            .arg("category:all:topics_reply")
            .arg(1)
            .arg(tid)
            .ignore()
            // add self post's time to category post_time sorted set. It's used to update category's post_count_new using zcount.
            .cmd("ZADD")
            .arg(&format!("category:{}:posts_time", cid))
            .arg(time)
            .arg(pid)
            .ignore();

        if let Some(pid) = post_id {
            // update other post's(the post self post replied to) last_reply_time perm field.
            pip.cmd("HSET")
                .arg(&format!("post:{}:set_perm", pid))
                .arg("last_reply_time")
                .arg(&time_string)
                .ignore()
                // update other post's reply count.
                .cmd("HINCRBY")
                .arg(&format!("post:{}:set_perm", pid))
                .arg("reply_count")
                .arg(1)
                .ignore()
                // update other post's reply count in topic's post_reply sorted set.
                .cmd("ZINCRBY")
                .arg(&format!("topic:{}:posts_reply", tid))
                .arg(1)
                .arg(LEX_BASE - pid)
                .ignore();
        }

        Poll::Ready(pip)
    }
}

struct AsyncPipelineBuild<'a, T>
where
    T: SelfIdString + RefTo<Vec<(&'a str, Vec<u8>)>>,
{
    slice: &'a [T],
    set_key: &'a [u8],
    should_expire: bool,
}

impl<'a, T> Future for AsyncPipelineBuild<'a, T>
where
    T: SelfIdString + RefTo<Vec<(&'a str, Vec<u8>)>>,
{
    type Output = Pipeline;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        let mut pip = pipe();
        pip.atomic();

        let mut key = Vec::with_capacity(28);
        key.extend_from_slice(self.set_key);

        for s in self.slice.iter() {
            let mut key = key.clone();
            key.extend_from_slice(s.self_id_string().as_bytes());
            key.extend_from_slice(SET_U8);

            let s = s.ref_to();

            pip.cmd("HMSET").arg(key.as_slice()).arg(s).ignore();
            if self.should_expire {
                pip.cmd("expire")
                    .arg(key.as_slice())
                    .arg(HASH_LIFE)
                    .ignore();
            }
        }

        Poll::Ready(pip)
    }
}

struct AsyncPipelineGet<'a> {
    ids: &'a [u32],
    set_key: &'a [u8],
    have_perm_fields: bool,
}

impl<'a> Future for AsyncPipelineGet<'a> {
    type Output = Pipeline;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        let mut pip = pipe();
        pip.atomic();

        let mut key = Vec::with_capacity(28);
        key.extend_from_slice(self.set_key);

        for i in self.ids.iter() {
            let mut key = key.clone();
            key.extend_from_slice(i.to_string().as_bytes());
            key.extend_from_slice(SET_U8);

            pip.cmd("HGETALL").arg(key.as_slice());
            if self.have_perm_fields {
                key.extend_from_slice(PERM_U8);
                pip.cmd("HGETALL").arg(key.as_slice());
            }
        }

        Poll::Ready(pip)
    }
}

//impl CacheService {
//    pub fn remove_category_cache_01(&self, cid: u32) -> impl Future01<Item=(), Error=ResError> {
//        // ToDo: future test the pipe lined cmd results
//        let mut pip = pipe();
//        pip.atomic();
//        pip.cmd("lrem")
//            .arg(cid)
//            .arg("category_id:meta")
//            .ignore()
//            .cmd("del")
//            .arg(&format!("category:{}:set", cid))
//            .ignore()
//            .cmd("del")
//            .arg(&format!("category:{}:topics_reply", cid))
//            .ignore()
//            .cmd("ZRANGE")
//            .arg(&format!("category:{}:topics_time", cid))
//            .arg(0)
//            .arg(-1);
//
//        pip.query_async(self.get_conn())
//            .map_err(ResError::from)
//            .and_then(move |(conn, tids): (MultiplexedConnection, Vec<u32>)| {
//                let mut pip = pipe();
//                pip.atomic();
//
//                for tid in tids.iter() {
//                    pip.cmd("del")
//                        .arg(&format!("topic:{}:set", tid))
//                        .ignore()
//                        .cmd("del")
//                        .arg(&format!("topic:{}:set_perm", tid))
//                        .ignore()
//                        .cmd("del")
//                        .arg(&format!("topic:{}:posts_reply", tid))
//                        .ignore()
//                        .cmd("lrange")
//                        .arg(&format!("topic:{}:list", tid))
//                        .arg(0)
//                        .arg(-1)
//                        .cmd("del")
//                        .arg(&format!("topic:{}:list", tid))
//                        .ignore();
//                }
//                pip.query_async(conn).map_err(ResError::from).and_then(
//                    |(conn, pids): (MultiplexedConnection, Vec<u32>)| {
//                        let mut pip = pipe();
//                        pip.atomic();
//
//                        for pid in pids.iter() {
//                            pip.cmd("del")
//                                .arg(&format!("post:{}:set", pid))
//                                .ignore()
//                                .cmd("del")
//                                .arg(&format!("post:{}:set_perm", pid))
//                                .ignore();
//                        }
//                        pip.query_async(conn)
//                            .map_err(ResError::from)
//                            .map(|(_, ())| ())
//                    },
//                )
//            })
//    }
//}

// helper functions for startup.
pub(crate) async fn build_hmsets_fn<'a, T>(
    conn: &'a mut MultiplexedConnection,
    vec: &'a [T],
    set_key: &'a [u8],
    should_expire: bool,
) -> Result<(), ResError>
where
    T: SelfIdString + RefTo<Vec<(&'a str, Vec<u8>)>>,
{
    let mut pip = pipe();
    pip.atomic();

    let mut key = Vec::with_capacity(28);
    key.extend_from_slice(set_key);

    for v in vec.iter() {
        let mut key = key.clone();
        key.extend_from_slice(v.self_id_string().as_bytes());
        key.extend_from_slice(SET_U8);

        let v = v.ref_to();

        pip.cmd("HMSET").arg(key.as_slice()).arg(v).ignore();
        if should_expire {
            pip.cmd("expire")
                .arg(key.as_slice())
                .arg(HASH_LIFE)
                .ignore();
        }
    }
    pip.query_async(conn)
        .err_into()
        .map_ok(|()| println!("updating cache"))
        .await
}

pub(crate) async fn build_list(
    conn: &mut MultiplexedConnection,
    vec: Vec<u32>,
    key: String,
) -> Result<(), ResError> {
    let mut pip = pipe();
    pip.atomic();

    pip.cmd("del").arg(key.as_str()).ignore();

    if !vec.is_empty() {
        pip.cmd("rpush").arg(key.as_str()).arg(vec).ignore();
    }

    pip.query_async(conn).err_into().await
}

pub(crate) async fn build_users_cache(
    vec: Vec<User>,
    conn: &mut MultiplexedConnection,
) -> Result<(), ResError> {
    let mut pip = pipe();
    pip.atomic();
    for v in vec.into_iter() {
        let key = format!("user:{}:set", v.self_id());
        let key_perm = format!("user:{}:set_perm", v.self_id());
        let v: Vec<(&str, Vec<u8>)> = v.ref_to();

        pip.cmd("HMSET")
            .arg(key.as_str())
            .arg(v)
            .ignore()
            .cmd("HMSET")
            .arg(key_perm.as_str())
            .arg(&[("online_status", 0)])
            .ignore();
    }
    pip.query_async(conn).err_into().await
}

pub(crate) async fn build_topics_cache_list(
    is_init: bool,
    vec: Vec<(u32, u32, Option<u32>, NaiveDateTime)>,
    conn: &mut MultiplexedConnection,
) -> Result<(), ResError> {
    let mut pip = pipe();
    pip.atomic();

    for (tid, cid, count, time) in vec.into_iter() {
        // only build these two zrange when init a new database.
        if is_init {
            let time = time.timestamp_millis();
            // ToDo: query existing cache for topic's real last reply time.
            pip.cmd("ZADD")
                .arg("category:all:topics_time")
                .arg(time)
                .arg(tid)
                .ignore()
                .cmd("ZADD")
                .arg("category:all:topics_reply")
                .arg(count.unwrap_or(0))
                .arg(tid)
                .ignore()
                .cmd("ZADD")
                .arg(&format!("category:{}:topics_time", cid))
                .arg(time)
                .arg(tid)
                .ignore()
                .cmd("ZADD")
                .arg(&format!("category:{}:topics_reply", cid))
                .arg(count.unwrap_or(0))
                .arg(tid)
                .ignore();
        }
        if let Some(count) = count {
            pip.cmd("HSET")
                .arg(&format!("topic:{}:set_perm", tid))
                .arg("reply_count")
                .arg(count)
                .ignore();
        }
    }

    pip.query_async(conn).err_into().await
}

pub(crate) async fn build_posts_cache_list(
    is_init: bool,
    vec: Vec<(u32, u32, Option<u32>, NaiveDateTime)>,
    conn: &mut MultiplexedConnection,
) -> Result<(), ResError> {
    let mut pipe = pipe();
    pipe.atomic();

    for (tid, pid, count, time) in vec.into_iter() {
        // only build these two zrange when init a new database.
        if is_init {
            let time = time.timestamp_millis();
            pipe.cmd("ZADD")
                .arg(&format!("topic:{}:posts_time_created", tid))
                .arg(time)
                .arg(pid)
                .ignore();
        }

        pipe.cmd("ZADD")
            .arg(&format!("topic:{}:posts_reply", tid))
            .arg(count.unwrap_or(0))
            .arg(LEX_BASE - pid)
            .ignore();

        if let Some(count) = count {
            pipe.cmd("HSET")
                .arg(&format!("post:{}:set_perm", pid))
                .arg("reply_count")
                .arg(count)
                .ignore();
        }
    }

    pipe.query_async(conn).err_into().await
}

pub(crate) fn clear_cache(redis_url: &str) -> Result<(), ResError> {
    let client = redis::Client::open(redis_url).expect("failed to connect to redis server");
    let mut conn = client
        .get_connection()
        .expect("failed to get redis connection");
    Ok(redis::cmd("flushall").query(&mut conn)?)
}
