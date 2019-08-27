use std::cell::RefCell;
use std::future::Future;

use actix::Recipient;
use chrono::NaiveDateTime;
use futures01::{
    Future as Future01,
    IntoFuture as IntoFuture01,
};
use futures::compat::Future01CompatExt;
use redis::{Client, cmd, pipe};
use redis::aio::SharedConnection;

use crate::handler::cache_update::CacheFailedMessage;
use crate::model::{
    cache_schema::{HashMapBrown, RefTo},
    category::Category,
    common::{BoxedFuture01Result, PinedBoxFutureResult, SelfId, SelfIdString, SelfUserId},
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

pub struct CacheService {
    pub url: String,
    pub cache: RefCell<SharedConnection>,
    pub recipient: Recipient<CacheFailedMessage>,
}

pub async fn connect_cache(redis_url: &str) -> Result<Option<SharedConnection>, ResError> {
    let executor = crate::util::executor_compat::Executor03As01::new(
        tokio_executor::DefaultExecutor::current(),
    );

    let conn = redis::Client::open(redis_url)?
        .get_shared_async_connection_with_executor(executor)
        .compat()
        .await?;

    Ok(Some(conn))
}

impl CacheService {
    pub async fn init(
        redis_url: &str,
        recipient: Recipient<CacheFailedMessage>,
    ) -> Result<CacheService, ResError> {
        let url = redis_url.to_owned();
        let cache = connect_cache(redis_url)
            .await?
            .ok_or(ResError::RedisConnection)?;

        Ok(CacheService {
            url,
            cache: RefCell::new(cache),
            recipient,
        })
    }

    pub fn add_activation_mail_01(
        &self,
        uid: u32,
        uuid: String,
        mail: String,
    ) -> impl Future01<Item=(), Error=ResError> {
        cmd("ZCOUNT")
            .arg("mail_queue")
            .arg(uid)
            .arg(uid)
            .query_async(self.get_conn())
            .map_err(ResError::from)
            .and_then(move |(conn, count): (_, usize)| {
                if count > 0 {
                    return futures01::future::Either::A(futures01::future::ok(()));
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

                futures01::future::Either::B(
                    pip.query_async(conn)
                        .map_err(ResError::from)
                        .map(|(_, ())| ()),
                )
            })
    }

    pub fn get_cache_with_uids_from_list<T>(
        &self,
        list_key: &str,
        page: usize,
        set_key: &'static [u8],
    ) -> impl Future<Output=Result<(Vec<T>, Vec<u32>), ResError>>
        where
            T: std::marker::Send + redis::FromRedisValue + SelfUserId + 'static,
    {
        let start = (page - 1) * 20;
        let end = start + LIMIT - 1;
        self.ids_from_cache_list_01(list_key, start, end)
            .and_then(move |(conn, ids)| {
                Self::from_cache_with_perm_with_uids_01(conn, ids, set_key)
            })
            .compat()
    }

    pub fn get_cache_with_uids_from_zrevrange_reverse_lex<T>(
        &self,
        zrange_key: &str,
        page: usize,
        set_key: &'static [u8],
    ) -> impl Future<Output=Result<(Vec<T>, Vec<u32>), ResError>>
        where
            T: std::marker::Send + redis::FromRedisValue + SelfUserId + 'static,
    {
        self.cache_with_uids_from_zrange(zrange_key, page, set_key, true, true)
    }

    pub fn get_cache_with_uids_from_zrevrange<T>(
        &self,
        zrange_key: &str,
        page: usize,
        set_key: &'static [u8],
    ) -> impl Future<Output=Result<(Vec<T>, Vec<u32>), ResError>>
        where
            T: std::marker::Send + redis::FromRedisValue + SelfUserId + 'static,
    {
        self.cache_with_uids_from_zrange(zrange_key, page, set_key, true, false)
    }

    pub fn get_cache_with_uids_from_zrange<T>(
        &self,
        zrange_key: &str,
        page: usize,
        set_key: &'static [u8],
    ) -> impl Future<Output=Result<(Vec<T>, Vec<u32>), ResError>>
        where
            T: std::marker::Send + redis::FromRedisValue + SelfUserId + 'static,
    {
        self.cache_with_uids_from_zrange(zrange_key, page, set_key, false, false)
    }

    fn cache_with_uids_from_zrange<T>(
        &self,
        zrange_key: &str,
        page: usize,
        set_key: &'static [u8],
        is_rev: bool,
        is_reverse_lex: bool,
    ) -> impl Future<Output=Result<(Vec<T>, Vec<u32>), ResError>>
        where
            T: std::marker::Send + redis::FromRedisValue + SelfUserId + 'static,
    {
        self.ids_from_cache_zrange_01(is_rev, zrange_key, (page - 1) * 20)
            .and_then(move |(conn, mut ids)| {
                if is_reverse_lex {
                    ids = ids.into_iter().map(|i| LEX_BASE - i).collect();
                }
                Self::from_cache_with_perm_with_uids_01(conn, ids, set_key)
            })
            .compat()
    }

    pub fn get_cache_with_uids_from_ids<T>(
        &self,
        ids: Vec<u32>,
        set_key: &'static [u8],
    ) -> impl Future<Output=Result<(Vec<T>, Vec<u32>), ResError>>
        where
            T: std::marker::Send + redis::FromRedisValue + SelfUserId + 'static,
    {
        Self::from_cache_with_perm_with_uids_01(self.get_conn(), ids, set_key).compat()
    }

    fn from_cache_with_perm_with_uids_01<T>(
        conn: SharedConnection,
        ids: Vec<u32>,
        set_key: &'static [u8],
    ) -> impl Future01<Item=(Vec<T>, Vec<u32>), Error=ResError>
        where
            T: std::marker::Send + redis::FromRedisValue + SelfUserId + 'static,
    {
        Self::from_cache_01(conn, ids, set_key, true).map(|t: Vec<T>| {
            let len = t.len();
            let mut uids = Vec::with_capacity(len);
            for t in t.iter() {
                uids.push(t.get_user_id());
            }
            (t, uids)
        })
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
//            .and_then(move |(conn, tids): (SharedConnection, Vec<u32>)| {
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
//                    |(conn, pids): (SharedConnection, Vec<u32>)| {
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

impl GetSharedConn for CacheService {
    fn get_conn(&self) -> SharedConnection {
        self.cache.borrow().clone()
    }
}

impl IdsFromList for CacheService {}

impl IdsFromSortedSet for CacheService {}

impl HashMapBrownFromCache for CacheService {}

impl FromCacheSingle for CacheService {}

impl FromCache for CacheService {}

impl UsersFromCache for CacheService {}

impl CategoriesFromCache for CacheService {}

impl AddToQueue for CacheService {}

impl AddToCache for CacheService {}

impl DeleteCache for CacheService {}

pub trait GetSharedConn {
    fn get_conn(&self) -> SharedConnection;
}

/// ids from list and sorted set will return an error if the result is empty.
/// we assume no data can be found on database if we don't have according id in cache.
pub trait IdsFromList: GetSharedConn {
    fn ids_from_cache_list(
        &self,
        list_key: &str,
        start: usize,
        end: usize,
    ) -> PinedBoxFutureResult<(SharedConnection, Vec<u32>)> {
        Box::pin(self.ids_from_cache_list_01(list_key, start, end).compat())
    }

    fn ids_from_cache_list_01(
        &self,
        list_key: &str,
        start: usize,
        end: usize,
    ) -> BoxedFuture01Result<(SharedConnection, Vec<u32>)> {
        Box::new(
            cmd("lrange")
                .arg(list_key)
                .arg(start)
                .arg(end)
                .query_async(self.get_conn())
                .map_err(ResError::from)
                .and_then(count_ids),
        )
    }
}

trait IdsFromSortedSet: GetSharedConn {
    fn ids_from_cache_zrange_01(
        &self,
        is_rev: bool,
        list_key: &str,
        offset: usize,
    ) -> BoxedFuture01Result<(SharedConnection, Vec<u32>)> {
        let (cmd_key, start, end) = if is_rev {
            ("zrevrangebyscore", "+inf", "-inf")
        } else {
            ("zrangebyscore", "-inf", "+inf")
        };

        Box::new(
            cmd(cmd_key)
                .arg(list_key)
                .arg(start)
                .arg(end)
                .arg("LIMIT")
                .arg(offset)
                .arg(20)
                .query_async(self.get_conn())
                .map_err(ResError::from)
                .and_then(count_ids),
        )
    }
}

fn count_ids(
    (conn, ids): (SharedConnection, Vec<u32>),
) -> Result<(SharedConnection, Vec<u32>), ResError> {
    if ids.is_empty() {
        Err(ResError::NoContent)
    } else {
        Ok((conn, ids))
    }
}

pub trait HashMapBrownFromCache: GetSharedConn
{
    fn hash_map_brown_from_cache_01(
        &self,
        key: &str,
    ) -> BoxedFuture01Result<HashMapBrown<String, String>> {
        Box::new(
            cmd("HGETALL")
                .arg(key)
                .query_async(self.get_conn())
                .from_err()
                .map(|(_, hm)| hm),
        )
    }
}

pub trait FromCacheSingle: GetSharedConn {
    fn from_cache_single_01<T>(&self, key: &str, set_key: &str) -> BoxedFuture01Result<T>
        where
            T: std::marker::Send + redis::FromRedisValue + 'static,
    {
        Box::new(
            cmd("HGETALL")
                .arg(&format!("{}:{}:set", set_key, key))
                .query_async(self.get_conn())
                .from_err()
                .map(|(_, hm)| hm),
        )
    }
}

pub trait FromCache {
    fn from_cache_01<T>(
        conn: SharedConnection,
        ids: Vec<u32>,
        set_key: &'static [u8],
        have_perm_fields: bool,
        // return input ids so the following function can also include the ids when mapping error to ResError::IdsFromCache.
    ) -> BoxedFuture01Result<Vec<T>>
        where
            T: std::marker::Send + redis::FromRedisValue + 'static,
    {
        let mut pip = pipe();
        pip.atomic();

        let mut key = Vec::with_capacity(28);
        key.extend_from_slice(set_key);

        for i in ids.iter() {
            let mut key = key.clone();
            key.extend_from_slice(i.to_string().as_bytes());
            key.extend_from_slice(SET_U8);

            pip.cmd("HGETALL").arg(key.as_slice());
            if have_perm_fields {
                key.extend_from_slice(PERM_U8);
                pip.cmd("HGETALL").arg(key.as_slice());
            }
        }

        Box::new(
            pip.query_async(conn)
                .then(|r: Result<(_, Vec<T>), redis::RedisError>| match r {
                    Ok((_, v)) => {
                        if v.len() != ids.len() {
                            Err(ResError::IdsFromCache(ids))
                        } else {
                            Ok(v)
                        }
                    }
                    Err(_) => Err(ResError::IdsFromCache(ids)),
                }),
        )
    }
}

pub trait UsersFromCache: GetSharedConn + FromCache {
    fn users_from_cache_01(&self, uids: Vec<u32>) -> BoxedFuture01Result<Vec<User>> {
        Box::new(Self::from_cache_01::<User>(
            self.get_conn(),
            uids,
            USER_U8,
            true,
        ))
    }
}

pub trait CategoriesFromCache: FromCache + GetSharedConn + IdsFromList {
    fn categories_from_cache_01(&self) -> BoxedFuture01Result<Vec<Category>> {
        Box::new(
            self.ids_from_cache_list_01("category_id:meta", 0, 999)
                .and_then(|(conn, vec): (_, Vec<u32>)| {
                    Self::from_cache_01(conn, vec, CATEGORY_U8, false)
                }),
        )
    }
}

pub trait AddToCache: GetSharedConn {
    fn add_topic_cache_01(&self, t: &Topic) -> BoxedFuture01Result<()> {
        let mut pip = pipe();
        pip.atomic();

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

        Box::new(
            pip.query_async(self.get_conn())
                .map_err(ResError::from)
                .map(|(_, ())| ()),
        )
    }

    fn add_post_cache_01(&self, p: &Post) -> BoxedFuture01Result<()> {
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

        Box::new(
            pip.query_async(self.get_conn())
                .from_err()
                .map(|(_, ())| ()),
        )
    }

    fn add_category_cache_01(&self, c: &Category) -> BoxedFuture01Result<()> {
        let id = c.id;
        let c: Vec<(&str, Vec<u8>)> = c.ref_to();

        let mut pip = pipe();
        pip.atomic();

        pip.cmd("rpush")
            .arg("category_id:meta")
            .arg(id)
            .ignore()
            .cmd("HMSET")
            .arg(&format!("category:{}:set", id))
            .arg(c)
            .ignore();

        Box::new(
            pip.query_async(self.get_conn())
                .map_err(ResError::from)
                .map(|(_, ())| ()),
        )
    }

    fn bulk_add_update_cache_01(
        &self,
        t: Vec<&Topic>,
        p: Vec<&Post>,
        u: Vec<&User>,
        c: Vec<&Category>,
    ) -> BoxedFuture01Result<()> {
        let mut pip = pipe();
        pip.atomic();

        let mut key = Vec::with_capacity(28);
        key.extend_from_slice(TOPIC_U8);

        for t in t.into_iter() {
            let mut key = key.clone();
            key.extend_from_slice(t.self_id_string().as_bytes());
            key.extend_from_slice(SET_U8);

            let t: Vec<(&'static str, Vec<u8>)> = t.ref_to();

            pip.cmd("HMSET")
                .arg(key.as_slice())
                .arg(t)
                .ignore()
                .cmd("expire")
                .arg(key.as_slice())
                .arg(HASH_LIFE)
                .ignore();
        }

        let mut key = Vec::with_capacity(28);
        key.extend_from_slice(POST_U8);

        for p in p.into_iter() {
            let mut key = key.clone();
            key.extend_from_slice(p.self_id_string().as_bytes());
            key.extend_from_slice(SET_U8);

            let p: Vec<(&'static str, Vec<u8>)> = p.ref_to();

            pip.cmd("HMSET")
                .arg(key.as_slice())
                .arg(p)
                .ignore()
                .cmd("expire")
                .arg(key.as_slice())
                .arg(HASH_LIFE)
                .ignore();
        }

        let mut key = Vec::with_capacity(28);
        key.extend_from_slice(USER_U8);

        for u in u.into_iter() {
            let mut key = key.clone();
            key.extend_from_slice(u.self_id_string().as_bytes());
            key.extend_from_slice(SET_U8);

            let u: Vec<(&'static str, Vec<u8>)> = u.ref_to();

            pip.cmd("HMSET").arg(key.as_slice()).arg(u).ignore();
        }

        let mut key = Vec::with_capacity(28);
        key.extend_from_slice(CATEGORY_U8);

        for c in c.into_iter() {
            let mut key = key.clone();
            key.extend_from_slice(c.self_id_string().as_bytes());
            key.extend_from_slice(SET_U8);

            let c: Vec<(&'static str, Vec<u8>)> = c.ref_to();

            pip.cmd("HMSET").arg(key.as_slice()).arg(c).ignore();
        }

        Box::new(
            pip.query_async(self.get_conn())
                .map_err(ResError::from)
                .map(|(_, ())| ()),
        )
    }
}

pub trait GetQueue: GetSharedConn {
    fn get_queue_01(&self, key: &str) -> BoxedFuture01Result<String> {
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
        Box::new(
            pip.query_async(self.get_conn())
                .map_err(ResError::from)
                .and_then(|(_, (mut s, ())): (_, (Vec<String>, _))| {
                    s.pop().ok_or(ResError::NoCache)
                }),
        )
    }
}

pub trait AddToQueue: GetSharedConn {
    fn add_to_queue_01(&self, key: &str, score: i64) -> BoxedFuture01Result<()> {
        Box::new(
            cmd("ZADD")
                .arg("psn_queue")
                .arg(score)
                .arg(key)
                .query_async(self.get_conn())
                .map_err(ResError::from)
                .map(|(_, ())| ()),
        )
    }
}

pub trait DeleteCache: GetSharedConn {
    fn del_cache_01(&self, key: &str) -> BoxedFuture01Result<()> {
        Box::new(
            cmd("del")
                .arg(key)
                .query_async(self.get_conn())
                .map_err(ResError::from)
                .map(|(_, ())| ()),
        )
    }
}

/// redis connection is only checked on insert request.
/// Connections are not shared between threads so the recovery will happen separately.
pub trait CheckCacheConn: GetSharedConn {
    fn check_cache_conn(&self) -> PinedBoxFutureResult<Option<SharedConnection>> {
        Box::pin(self.check_cache_conn_01().compat())
    }

    fn check_cache_conn_01(&self) -> BoxedFuture01Result<Option<SharedConnection>> {
        let url = self.self_url();
        Box::new(
            redis::cmd("PING")
                .query_async(self.get_conn())
                .map(|(_, ())| None)
                .or_else(move |_| {
                    Client::open(url.as_str())
                        .into_future()
                        .and_then(|c| c.get_shared_async_connection().map(Some))
                })
                .map_err(ResError::from),
        )
    }

    fn if_replace_cache(&self, opt: Option<SharedConnection>) -> &Self {
        if let Some(c) = opt {
            self.replace_cache(c);
        }
        self
    }

    fn self_url(&self) -> String;

    fn replace_cache(&self, c: SharedConnection);
}

impl CheckCacheConn for CacheService {
    fn self_url(&self) -> String {
        self.url.to_owned()
    }

    fn replace_cache(&self, c: SharedConnection) {
        self.cache.replace(c);
    }
}

pub fn build_hmsets_01<T>(
    conn: SharedConnection,
    vec: &[T],
    set_key: &'static [u8],
    should_expire: bool,
) -> impl Future01<Item=(), Error=ResError>
    where
        T: SelfIdString + RefTo<Vec<(&'static str, Vec<u8>)>>,
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
        .map_err(|e| {
            println!("{:?}", e);
            ResError::from(e)
        })
        .map(|(_, ())| println!("updating cache"))
}

// helper functions for build cache when server start.
pub fn build_list(
    conn: SharedConnection,
    vec: Vec<u32>,
    key: String,
) -> impl Future01<Item=(), Error=ResError> {
    let mut pip = pipe();
    pip.atomic();

    pip.cmd("del").arg(key.as_str()).ignore();

    if !vec.is_empty() {
        pip.cmd("rpush").arg(key.as_str()).arg(vec).ignore();
    }

    pip.query_async(conn)
        .map_err(ResError::from)
        .map(|(_, ())| ())
}

pub fn build_users_cache_01(
    vec: Vec<User>,
    conn: SharedConnection,
) -> impl Future01<Item=(), Error=ResError> {
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
    pip.query_async(conn)
        .map_err(ResError::from)
        .map(|(_, ())| ())
}

pub fn build_topics_cache_list_01(
    is_init: bool,
    vec: Vec<(u32, u32, Option<u32>, NaiveDateTime)>,
    conn: SharedConnection,
) -> impl Future01<Item=(), Error=ResError> {
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

    pip.query_async(conn)
        .map_err(ResError::from)
        .map(|(_, ())| ())
}

pub fn build_posts_cache_list_01(
    is_init: bool,
    vec: Vec<(u32, u32, Option<u32>, NaiveDateTime)>,
    conn: SharedConnection,
) -> impl Future01<Item=(), Error=ResError> {
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

    pipe.query_async(conn)
        .map_err(ResError::from)
        .map(|(_, ())| ())
}

pub fn clear_cache(redis_url: &str) -> Result<(), ResError> {
    let client = redis::Client::open(redis_url).expect("failed to connect to redis server");
    let mut conn = client
        .get_connection()
        .expect("failed to get redis connection");
    Ok(redis::cmd("flushall").query(&mut conn)?)
}
