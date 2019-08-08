use futures::future::{join_all, ok as ft_ok, Either};
use std::{collections::HashMap, time::Duration};

use actix::prelude::{ActorFuture, AsyncContext, Context, Future, WrapFuture};
use chrono::{NaiveDateTime, Utc};
use redis::{
    aio::SharedConnection, cmd, from_redis_value, pipe, Client, ErrorKind, FromRedisValue,
    RedisResult, Value,
};

use crate::model::actors::TalkService;
use crate::model::{
    actors::SharedConn,
    category::Category,
    common::{GetSelfId, GetUserId},
    errors::ResError,
    post::Post,
    topic::Topic,
    user::User,
};
use crate::{CacheUpdateService, MessageService};

// page offsets of list query
const LIMIT: isize = 20;
// use LEX_BASE minus pid and tid before adding to zrange.
const LEX_BASE: u32 = std::u32::MAX;
// list_pop update interval time gap in seconds
const LIST_TIME_GAP: Duration = Duration::from_secs(10);
// hash life is expire time of topic and post hash in seconds.
const HASH_LIFE: usize = 172800;
// mail life is expire time of mail hash in seconds
const MAIL_LIFE: usize = 3600;

pub struct CacheService {
    pub cache: SharedConnection,
}

impl CacheService {
    pub fn init(redis_url: &str) -> impl Future<Item = CacheService, Error = ()> {
        Client::open(redis_url)
            .unwrap_or_else(|e| panic!("{:?}", e))
            .get_shared_async_connection()
            .map_err(|e| panic!("{:?}", e))
            .map(|c| CacheService { cache: c })
    }
}

impl GetSharedConn for CacheService {
    fn get_conn(&self) -> SharedConnection {
        self.cache.clone()
    }
}

impl CacheService {
    pub fn update_users(&self, u: Vec<User>) {
        actix_rt::spawn(build_hmsets(self.get_conn(), u, "user", false));
    }

    pub fn update_categories(&self, u: Vec<Category>) {
        actix_rt::spawn(build_hmsets(self.get_conn(), u, "category", false));
    }

    pub fn update_topics(&self, t: Vec<Topic>) {
        actix_rt::spawn(build_hmsets(self.get_conn(), t, "topic", true));
    }

    pub fn update_posts(&self, t: Vec<Post>) {
        actix_rt::spawn(build_hmsets(self.get_conn(), t, "post", true));
    }

    pub fn get_hash_map(
        &self,
        key: &str,
    ) -> impl Future<Item = HashMap<String, String>, Error = ResError> {
        self.hash_map_from_cache(key)
    }

    pub fn get_cache_with_uids_from_list<T>(
        &self,
        list_key: &str,
        page: i64,
        set_key: &'static str,
    ) -> impl Future<Item = (Vec<T>, Vec<u32>), Error = ResError>
    where
        T: std::marker::Send
            + redis::FromRedisValue
            + AttachPermFields<Result = T>
            + GetUserId
            + 'static,
    {
        let start = (page as isize - 1) * 20;
        let end = start + LIMIT - 1;
        self.ids_from_cache_list(list_key, start, end)
            .and_then(move |(conn, ids)| Self::from_cache_with_perm(conn, ids, set_key))
    }

    pub fn get_cache_with_uids_from_zrevrange_reverse_lex<T>(
        &self,
        zrange_key: &str,
        page: i64,
        set_key: &'static str,
    ) -> impl Future<Item = (Vec<T>, Vec<u32>), Error = ResError>
    where
        T: std::marker::Send
            + redis::FromRedisValue
            + AttachPermFields<Result = T>
            + GetUserId
            + 'static,
    {
        self.cache_with_uids_from_zrange(zrange_key, page, set_key, true, true)
    }

    pub fn get_cache_with_uids_from_zrevrange<T>(
        &self,
        zrange_key: &str,
        page: i64,
        set_key: &'static str,
    ) -> impl Future<Item = (Vec<T>, Vec<u32>), Error = ResError>
    where
        T: std::marker::Send
            + redis::FromRedisValue
            + AttachPermFields<Result = T>
            + GetUserId
            + 'static,
    {
        self.cache_with_uids_from_zrange(zrange_key, page, set_key, true, false)
    }

    pub fn get_cache_with_uids_from_zrange<T>(
        &self,
        zrange_key: &str,
        page: i64,
        set_key: &'static str,
    ) -> impl Future<Item = (Vec<T>, Vec<u32>), Error = ResError>
    where
        T: std::marker::Send
            + redis::FromRedisValue
            + AttachPermFields<Result = T>
            + GetUserId
            + 'static,
    {
        self.cache_with_uids_from_zrange(zrange_key, page, set_key, false, false)
    }

    fn cache_with_uids_from_zrange<T>(
        &self,
        zrange_key: &str,
        page: i64,
        set_key: &'static str,
        is_rev: bool,
        is_reverse_lex: bool,
    ) -> impl Future<Item = (Vec<T>, Vec<u32>), Error = ResError>
    where
        T: std::marker::Send
            + redis::FromRedisValue
            + AttachPermFields<Result = T>
            + GetUserId
            + 'static,
    {
        self.ids_from_cache_zrange(is_rev, zrange_key, ((page - 1) * 20) as usize)
            .and_then(move |(conn, mut ids)| {
                if is_reverse_lex {
                    ids = ids.into_iter().map(|i| LEX_BASE - i).collect();
                }
                Self::from_cache_with_perm(conn, ids, set_key)
            })
    }

    pub fn get_cache_with_uids_from_ids<T>(
        &self,
        ids: Vec<u32>,
        set_key: &str,
    ) -> impl Future<Item = (Vec<T>, Vec<u32>), Error = ResError>
    where
        T: std::marker::Send
            + redis::FromRedisValue
            + AttachPermFields<Result = T>
            + GetUserId
            + 'static,
    {
        Self::from_cache_with_perm(self.get_conn(), ids, set_key)
    }

    pub fn add_topic(&self, t: Topic) {
        let mut pip = pipe();
        pip.atomic();

        let tid = t.id;
        let cid = t.category_id;
        let time = t.created_at.timestamp_millis();
        let key = format!("topic:{}:set", t.self_id());
        let t: Vec<(&str, String)> = t.into();

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

        actix_rt::spawn(
            pip.query_async(self.get_conn())
                //ToDo: add error handling
                .map_err(|_| ())
                .map(|(_, ())| ()),
        )
    }

    pub fn add_post(&self, p: Post) {
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
        let p: Vec<(&str, String)> = p.into();

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

        actix_rt::spawn(
            pip.query_async(self.get_conn())
                .map_err(|_| ())
                .map(|(_, ())| ()),
        )
    }

    pub fn add_category(&self, c: Category) {
        let id = c.id;
        let c: Vec<(&str, String)> = c.into();

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

        actix_rt::spawn(
            pip.query_async(self.get_conn())
                .map_err(|_| ())
                .map(|(_, ())| ()),
        )
    }

    pub fn add_activation_mail_self(
        &self,
        uid: u32,
        uuid: String,
        mail: String,
    ) -> impl Future<Item = (), Error = ()> {
        cmd("ZCOUNT")
            .arg("mail_queue")
            .arg(uid)
            .arg(uid)
            .query_async(self.get_conn())
            .map_err(|_| ())
            .and_then(move |(conn, count): (_, usize)| {
                if count > 0 {
                    return Either::A(ft_ok(()));
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

                Either::B(pip.query_async(conn).map_err(|_| ()).map(|(_, ())| ()))
            })
    }
}

impl TalkService {
    pub fn set_online_status(
        &self,
        uid: u32,
        status: u32,
        set_last_online_time: bool,
    ) -> impl Future<Item = (), Error = ResError> {
        let mut arg = Vec::with_capacity(2);
        arg.push(("online_status", status.to_string()));

        if set_last_online_time {
            arg.push(("last_online", Utc::now().naive_utc().to_string()))
        }

        cmd("HMSET")
            .arg(&format!("user:{}:set", uid))
            .arg(arg)
            .query_async(self.get_conn())
            .from_err()
            .map(|(_, ())| ())
    }

    pub fn get_users_cache_from_ids(
        &self,
        uids: Vec<u32>,
    ) -> impl Future<Item = Vec<User>, Error = ResError> {
        self.users_from_cache(uids)
    }
}

impl GetSharedConn for TalkService {
    fn get_conn(&self) -> SharedConn {
        self.cache.clone()
    }
}

impl MessageService {
    pub fn get_queue(&self, key: &str) -> impl Future<Item = String, Error = ResError> {
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
        pip.query_async(self.get_conn())
            .from_err()
            .and_then(|(_, (mut s, ())): (_, (Vec<String>, _))| s.pop().ok_or(ResError::NoCache))
    }
}

impl CacheUpdateService {
    pub fn start_interval(&mut self, ctx: &mut Context<Self>) {
        self.update_list_pop(ctx);
    }

    fn update_list_pop(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(LIST_TIME_GAP, move |act, ctx| {
            ctx.spawn(
                act.categories_from_cache()
                    .into_actor(act)
                    .map_err(|_, _, _| ())
                    .and_then(|cat, act, _| {
                        let conn = act.get_conn();
                        let yesterday = Utc::now().naive_utc().timestamp_millis() - 86400000;
                        let mut vec = Vec::new();

                        for c in cat.iter() {
                            // update_list will also update topic count new.
                            vec.push(Either::A(update_list(Some(c.id), yesterday, conn.clone())));
                            vec.push(Either::B(update_post_count(c.id, yesterday, conn.clone())));
                        }
                        vec.push(Either::A(update_list(None, yesterday, conn)));

                        join_all(vec)
                            .into_actor(act)
                            .map_err(|_, _, _| ())
                            .map(|_, _, _| ())
                    }),
            );
        });
    }
}

pub trait GetSharedConn {
    fn get_conn(&self) -> SharedConn;
}

impl GetSharedConn for CacheUpdateService {
    fn get_conn(&self) -> SharedConn {
        self.cache.as_ref().unwrap().clone()
    }
}

impl GetSharedConn for MessageService {
    fn get_conn(&self) -> SharedConn {
        self.cache.as_ref().unwrap().clone()
    }
}

fn count_ids((conn, ids): (SharedConn, Vec<u32>)) -> Result<(SharedConn, Vec<u32>), ResError> {
    if ids.len() == 0 {
        Err(ResError::NoContent)
    } else {
        Ok((conn, ids))
    }
}

pub trait IdsFromList
where
    Self: GetSharedConn,
{
    fn ids_from_cache_list(
        &self,
        list_key: &str,
        start: isize,
        end: isize,
    ) -> Box<dyn Future<Item = (SharedConn, Vec<u32>), Error = ResError>> {
        Box::new(
            cmd("lrange")
                .arg(list_key)
                .arg(start)
                .arg(end)
                .query_async(self.get_conn())
                .from_err()
                .and_then(count_ids),
        )
    }
}

impl IdsFromList for CacheUpdateService {}

impl IdsFromList for CacheService {}

trait IdsFromSortedSet
where
    Self: GetSharedConn,
{
    fn ids_from_cache_zrange(
        &self,
        is_rev: bool,
        list_key: &str,
        offset: usize,
    ) -> Box<dyn Future<Item = (SharedConn, Vec<u32>), Error = ResError>> {
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
                .from_err()
                .and_then(count_ids),
        )
    }
}

impl IdsFromSortedSet for CacheService {}

trait HashMapFromCache
where
    Self: GetSharedConn,
{
    fn hash_map_from_cache(
        &self,
        key: &str,
    ) -> Box<dyn Future<Item = HashMap<String, String>, Error = ResError>> {
        Box::new(
            cmd("HGETALL")
                .arg(key)
                .query_async(self.get_conn())
                .from_err()
                .map(|(_, hm)| hm),
        )
    }
}

impl HashMapFromCache for CacheService {}

pub trait DeleteCache
where
    Self: GetSharedConn,
{
    fn del_cache(&self, key: &str) -> Box<dyn Future<Item = (), Error = ResError>> {
        Box::new(
            cmd("del")
                .arg(key)
                .query_async(self.get_conn())
                .map(|(_, ())| ())
                .from_err(),
        )
    }
}

impl DeleteCache for CacheService {}

pub trait FromCache {
    fn from_cache<T>(
        conn: SharedConn,
        ids: Vec<u32>,
        set_key: &str,
        // return input ids so the following function can also include the ids when mapping error to ResError::IdsFromCache.
    ) -> Box<dyn Future<Item = Vec<T>, Error = ResError>>
    where
        T: std::marker::Send + redis::FromRedisValue + 'static,
    {
        let mut pip = pipe();
        pip.atomic();

        for i in ids.iter() {
            pip.cmd("HGETALL").arg(&format!("{}:{}:set", set_key, i));
        }

        Box::new(pip.query_async(conn).then(|r| match r {
            Ok((_, v)) => Ok(v),
            Err(_) => Err(ResError::IdsFromCache(ids)),
        }))
    }
}

impl FromCache for CacheService {}

impl FromCache for CacheUpdateService {}

impl FromCache for TalkService {}

trait FromCacheWithPerm {
    fn from_cache_with_perm<T>(
        conn: SharedConn,
        ids: Vec<u32>,
        set_key: &str,
    ) -> Box<dyn Future<Item = (Vec<T>, Vec<u32>), Error = ResError>>
    where
        T: std::marker::Send
            + redis::FromRedisValue
            + AttachPermFields<Result = T>
            + GetUserId
            + 'static,
    {
        let mut pip = pipe();
        pip.atomic();

        for i in ids.iter() {
            pip.cmd("HGETALL")
                .arg(&format!("{}:{}:set", set_key, i))
                .cmd("HGETALL")
                .arg(&format!("{}:{}:set_perm", set_key, i));
        }

        Box::new(pip.query_async(conn).then(
            move |r: Result<(_, Vec<(T, HashMap<String, String>)>), _>| match r {
                Err(_) => Err(ResError::IdsFromCache(ids)),
                Ok((_, hm)) => {
                    let len = hm.len();
                    let mut v = Vec::with_capacity(len);
                    let mut uids = Vec::with_capacity(len);
                    for (t, h) in hm.into_iter() {
                        uids.push(t.get_user_id());
                        v.push(t.attach_perm_fields(&h));
                    }
                    Ok((v, uids))
                }
            },
        ))
    }
}

impl FromCacheWithPerm for CacheService {}

pub trait AttachPermFields {
    type Result;
    fn attach_perm_fields(self, h: &HashMap<String, String>) -> Self::Result;
}

impl AttachPermFields for Topic {
    type Result = Topic;
    fn attach_perm_fields(mut self, h: &HashMap<String, String>) -> Self::Result {
        self.last_reply_time = match h.get("last_reply_time") {
            Some(t) => NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S%.f").ok(),
            None => None,
        };
        self.reply_count = match h.get("reply_count") {
            Some(t) => t.parse::<u32>().ok(),
            None => None,
        };
        self
    }
}

impl AttachPermFields for Post {
    type Result = Post;
    fn attach_perm_fields(mut self, h: &HashMap<String, String>) -> Self::Result {
        self.last_reply_time = match h.get("last_reply_time") {
            Some(t) => NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S%.f").ok(),
            None => None,
        };
        self.reply_count = match h.get("reply_count") {
            Some(t) => t.parse::<u32>().ok(),
            None => None,
        };
        self
    }
}

pub trait UsersFromCache
where
    Self: FromCache + GetSharedConn,
{
    fn users_from_cache(
        &self,
        uids: Vec<u32>,
    ) -> Box<dyn Future<Item = Vec<User>, Error = ResError>> {
        Box::new(Self::from_cache(self.get_conn(), uids, "user"))
    }
}

impl UsersFromCache for TalkService {}

impl UsersFromCache for CacheService {}

pub trait CategoriesFromCache
where
    Self: FromCache + GetSharedConn + IdsFromList,
{
    fn categories_from_cache(&self) -> Box<dyn Future<Item = Vec<Category>, Error = ResError>> {
        Box::new(
            self.ids_from_cache_list("category_id:meta", 0, -1)
                .and_then(|(conn, vec): (_, Vec<u32>)| Self::from_cache(conn, vec, "category")),
        )
    }
}

impl CategoriesFromCache for CacheService {}

impl CategoriesFromCache for CacheUpdateService {}

trait ParseFromRedisValue {
    type Result;
    fn parse_from_redis_value(v: &Value) -> Result<Self::Result, redis::RedisError>
    where
        Self::Result: Default,
    {
        match *v {
            Value::Bulk(ref items) => {
                if items.is_empty() {
                    return Err((ErrorKind::ResponseError, "Response is empty"))?;
                }
                let mut t = Self::Result::default();
                let mut iter = items.iter();
                loop {
                    let k = match iter.next() {
                        Some(v) => v,
                        None => break,
                    };
                    let v = match iter.next() {
                        Some(v) => v,
                        None => break,
                    };
                    let key: String = from_redis_value(k)?;
                    let _ = Self::parse_pattern(&mut t, key.as_str(), v)?;
                }
                Ok(t)
            }
            _ => return Err((ErrorKind::ResponseError, "Response type not compatible"))?,
        }
    }

    fn parse_pattern(t: &mut Self::Result, key: &str, v: &Value) -> Result<(), redis::RedisError>;
}

impl ParseFromRedisValue for Topic {
    type Result = Topic;
    fn parse_pattern(t: &mut Topic, k: &str, v: &Value) -> Result<(), redis::RedisError> {
        match k {
            "id" => t.id = from_redis_value::<u32>(v)?,
            "user_id" => t.user_id = from_redis_value::<u32>(v)?,
            "category_id" => t.category_id = from_redis_value::<u32>(v)?,
            "title" => t.title = from_redis_value::<String>(v)?,
            "body" => t.body = from_redis_value::<String>(v)?,
            "thumbnail" => t.thumbnail = from_redis_value::<String>(v)?,
            "created_at" => {
                t.created_at = NaiveDateTime::parse_from_str(
                    from_redis_value::<String>(v)?.as_str(),
                    "%Y-%m-%d %H:%M:%S%.f",
                )
                .map_err(|_| (ErrorKind::TypeError, "Invalid NaiveDateTime"))?
            }
            "updated_at" => {
                t.updated_at = NaiveDateTime::parse_from_str(
                    from_redis_value::<String>(v)?.as_str(),
                    "%Y-%m-%d %H:%M:%S%.f",
                )
                .map_err(|_| (ErrorKind::TypeError, "Invalid NaiveDateTime"))?
            }
            "is_locked" => {
                t.is_locked = if from_redis_value::<u8>(v)? == 0 {
                    false
                } else {
                    true
                }
            }
            "is_visible" => {
                t.is_visible = if from_redis_value::<u8>(v)? == 0 {
                    false
                } else {
                    true
                }
            }
            _ => return Err((ErrorKind::ResponseError, "Response type not compatible"))?,
        };
        Ok(())
    }
}

impl ParseFromRedisValue for Post {
    type Result = Post;
    fn parse_pattern(p: &mut Post, k: &str, v: &Value) -> Result<(), redis::RedisError> {
        match k {
            "id" => p.id = from_redis_value(v)?,
            "user_id" => p.user_id = from_redis_value(v)?,
            "topic_id" => p.topic_id = from_redis_value(v)?,
            "category_id" => p.category_id = from_redis_value(v)?,
            "post_id" => {
                p.post_id = match from_redis_value::<u32>(v).ok() {
                    Some(pid) => {
                        if pid == 0 {
                            None
                        } else {
                            Some(pid)
                        }
                    }
                    None => None,
                }
            }
            "post_content" => p.post_content = from_redis_value(v)?,
            "created_at" => {
                p.created_at = NaiveDateTime::parse_from_str(
                    from_redis_value::<String>(v)?.as_str(),
                    "%Y-%m-%d %H:%M:%S%.f",
                )
                .map_err(|_| (ErrorKind::TypeError, "Invalid NaiveDateTime"))?
            }
            "updated_at" => {
                p.updated_at = NaiveDateTime::parse_from_str(
                    from_redis_value::<String>(v)?.as_str(),
                    "%Y-%m-%d %H:%M:%S%.f",
                )
                .map_err(|_| (ErrorKind::TypeError, "Invalid NaiveDateTime"))?
            }
            //ToDo: change to boolean paring.
            "is_locked" => {
                p.is_locked = if from_redis_value::<u32>(v)? == 0 {
                    false
                } else {
                    true
                }
            }
            _ => return Err((ErrorKind::ResponseError, "Response type not compatible"))?,
        };
        Ok(())
    }
}

impl ParseFromRedisValue for User {
    type Result = User;
    fn parse_pattern(u: &mut User, k: &str, v: &Value) -> Result<(), redis::RedisError> {
        match k {
            "id" => u.id = from_redis_value::<u32>(v)?,
            "username" => u.username = from_redis_value(v)?,
            "email" => u.email = from_redis_value(v)?,
            "hashed_password" => (),
            "avatar_url" => u.avatar_url = from_redis_value(v)?,
            "signature" => u.signature = from_redis_value(v)?,
            "created_at" => {
                u.created_at = NaiveDateTime::parse_from_str(
                    from_redis_value::<String>(v)?.as_str(),
                    "%Y-%m-%d %H:%M:%S%.f",
                )
                .map_err(|_| (ErrorKind::TypeError, "Invalid NaiveDateTime"))?
            }
            "privilege" => u.privilege = from_redis_value(v)?,
            "show_email" => {
                u.show_email = from_redis_value::<String>(v)?
                    .parse::<bool>()
                    .map_err(|_| (ErrorKind::TypeError, "Invalid boolean"))?
            }
            "online_status" => u.online_status = from_redis_value::<u32>(v).ok(),
            "last_online" => {
                u.last_online = match from_redis_value::<String>(v).ok() {
                    Some(v) => {
                        NaiveDateTime::parse_from_str(v.as_str(), "%Y-%m-%d %H:%M:%S%.f").ok()
                    }
                    None => None,
                }
            }
            _ => return Err((ErrorKind::ResponseError, "Response type not compatible"))?,
        };
        Ok(())
    }
}

impl ParseFromRedisValue for Category {
    type Result = Category;
    fn parse_pattern(c: &mut Category, k: &str, v: &Value) -> Result<(), redis::RedisError> {
        match k {
            "id" => c.id = from_redis_value(v)?,
            "name" => c.name = from_redis_value(v)?,
            "thumbnail" => c.thumbnail = from_redis_value(v)?,
            "topic_count" => c.topic_count = from_redis_value(v).ok(),
            "post_count" => c.post_count = from_redis_value(v).ok(),
            "topic_count_new" => c.topic_count_new = from_redis_value(v).ok(),
            "post_count_new" => c.post_count_new = from_redis_value(v).ok(),
            _ => return Err((ErrorKind::ResponseError, "Response type not compatible"))?,
        };
        Ok(())
    }
}

impl FromRedisValue for Topic {
    fn from_redis_value(v: &Value) -> RedisResult<Topic> {
        Topic::parse_from_redis_value(v)
    }
}

impl FromRedisValue for Post {
    fn from_redis_value(v: &Value) -> RedisResult<Post> {
        Post::parse_from_redis_value(v)
    }
}

impl FromRedisValue for User {
    fn from_redis_value(v: &Value) -> RedisResult<User> {
        User::parse_from_redis_value(v)
    }
}

impl FromRedisValue for Category {
    fn from_redis_value(v: &Value) -> RedisResult<Category> {
        Category::parse_from_redis_value(v)
    }
}

impl Into<Vec<(&str, String)>> for User {
    fn into(self) -> Vec<(&'static str, String)> {
        vec![
            ("id", self.id.to_string()),
            ("username", self.username.to_owned()),
            ("email", self.email.to_string()),
            ("avatar_url", self.avatar_url.to_owned()),
            ("signature", self.signature.to_owned()),
            ("created_at", self.created_at.to_string()),
            ("privilege", self.privilege.to_string()),
            ("show_email", self.show_email.to_string()),
        ]
    }
}

impl Into<Vec<(&str, String)>> for Topic {
    fn into(self) -> Vec<(&'static str, String)> {
        vec![
            ("id", self.id.to_string()),
            ("user_id", self.user_id.to_string()),
            ("category_id", self.category_id.to_string()),
            ("title", self.title.to_owned()),
            ("body", self.body.to_owned()),
            ("thumbnail", self.thumbnail.to_owned()),
            ("created_at", self.created_at.to_string()),
            ("updated_at", self.updated_at.to_string()),
            (
                "is_locked",
                if self.is_locked == true {
                    "1".to_owned()
                } else {
                    "0".to_owned()
                },
            ),
            (
                "is_visible",
                if self.is_visible == true {
                    "1".to_owned()
                } else {
                    "0".to_owned()
                },
            ),
        ]
    }
}

impl Into<Vec<(&str, String)>> for Post {
    fn into(self) -> Vec<(&'static str, String)> {
        vec![
            ("id", self.id.to_string()),
            ("user_id", self.user_id.to_string()),
            ("topic_id", self.topic_id.to_string()),
            ("category_id", self.category_id.to_string()),
            ("post_id", self.post_id.unwrap_or(0).to_string()),
            ("post_content", self.post_content.to_owned()),
            ("created_at", self.created_at.to_string()),
            ("updated_at", self.updated_at.to_string()),
            (
                "is_locked",
                if self.is_locked == true {
                    "1".to_owned()
                } else {
                    "0".to_owned()
                },
            ),
        ]
    }
}

impl Into<Vec<(&str, String)>> for Category {
    fn into(self) -> Vec<(&'static str, String)> {
        vec![
            ("id", self.id.to_string()),
            ("name", self.name.to_owned()),
            ("thumbnail", self.thumbnail.to_owned()),
        ]
    }
}

pub fn build_hmsets<T>(
    conn: SharedConn,
    vec: Vec<T>,
    key: &'static str,
    should_expire: bool,
) -> impl Future<Item = (), Error = ()>
where
    T: GetSelfId + Into<Vec<(&'static str, String)>>,
{
    let mut pip = pipe();
    pip.atomic();
    for v in vec.into_iter() {
        let key = format!("{}:{}:set", key, v.self_id());
        let v: Vec<(&str, String)> = v.into();

        pip.cmd("HMSET").arg(key.as_str()).arg(v).ignore();
        if should_expire {
            pip.cmd("expire").arg(key.as_str()).arg(HASH_LIFE).ignore();
        }
    }
    pip.query_async(conn)
        .map_err(|_| ())
        .map(|(_, ())| println!("updating cache"))
}

impl CacheService {
    pub fn remove_category(&self, cid: u32) -> impl Future<Item = (), Error = ResError> {
        // ToDo: future test the pipe lined cmd results
        let mut pip = pipe();
        pip.atomic();
        pip.cmd("lrem")
            .arg(cid)
            .arg("category_id:meta")
            .ignore()
            .cmd("del")
            .arg(&format!("category:{}:set", cid))
            .ignore()
            .cmd("del")
            .arg(&format!("category:{}:topics_reply", cid))
            .ignore()
            .cmd("ZRANGE")
            .arg(&format!("category:{}:topics_time", cid))
            .arg(0)
            .arg(-1);

        pip.query_async(self.get_conn()).from_err().and_then(
            move |(conn, tids): (SharedConn, Vec<u32>)| {
                let mut pip = pipe();
                pip.atomic();

                for tid in tids.iter() {
                    pip.cmd("del")
                        .arg(&format!("topic:{}:set", tid))
                        .ignore()
                        .cmd("del")
                        .arg(&format!("topic:{}:set_perm", tid))
                        .ignore()
                        .cmd("del")
                        .arg(&format!("topic:{}:posts_reply", tid))
                        .ignore()
                        .cmd("lrange")
                        .arg(&format!("topic:{}:list", tid))
                        .arg(0)
                        .arg(-1)
                        .cmd("del")
                        .arg(&format!("topic:{}:list", tid))
                        .ignore();
                }
                pip.query_async(conn)
                    .from_err()
                    .and_then(|(conn, pids): (SharedConn, Vec<u32>)| {
                        let mut pip = pipe();
                        pip.atomic();

                        for pid in pids.iter() {
                            pip.cmd("del")
                                .arg(&format!("post:{}:set", pid))
                                .ignore()
                                .cmd("del")
                                .arg(&format!("post:{}:set_perm", pid))
                                .ignore();
                        }
                        pip.query_async(conn).from_err().map(|(_, ())| ())
                    })
            },
        )
    }
}

fn update_post_count(
    cid: u32,
    yesterday: i64,
    conn: SharedConn,
) -> impl Future<Item = (), Error = ResError> {
    let time_key = format!("category:{}:posts_time", cid);
    let set_key = format!("category:{}:set", cid);

    cmd("ZCOUNT")
        .arg(time_key.as_str())
        .arg(yesterday)
        .arg("+inf")
        .query_async(conn)
        .from_err()
        .and_then(move |(conn, count): (_, u32)| {
            cmd("HMSET")
                .arg(set_key.as_str())
                .arg(&[("post_count_new", count)])
                .query_async(conn)
                .from_err()
                .map(|(_, ())| ())
        })
}

fn update_list(
    cid: Option<u32>,
    yesterday: i64,
    conn: SharedConn,
) -> impl Future<Item = (), Error = ResError> {
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

    pip.query_async(conn).from_err().and_then(
        move |(conn, (tids, counts)): (_, (HashMap<u32, i64>, Vec<(u32, u32)>))| {
            let mut counts = counts
                .into_iter()
                .filter(|(tid, _)| tids.contains_key(tid))
                .collect::<Vec<(u32, u32)>>();

            use std::cmp::Ordering;
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

            let vec = counts.into_iter().map(|(id, _)| id).collect::<Vec<u32>>();

            let mut should_update = false;
            let mut pip = pipe();
            pip.atomic();

            if let Some(key) = set_key {
                pip.cmd("HSET")
                    .arg(&key)
                    .arg("topic_count_new")
                    .arg(tids.len())
                    .ignore();
                should_update = true;
            }

            if vec.len() > 0 {
                pip.cmd("del")
                    .arg(&list_key)
                    .ignore()
                    .cmd("rpush")
                    .arg(&list_key)
                    .arg(vec)
                    .ignore();
                should_update = true;
            }

            if should_update {
                Either::A(pip.query_async(conn).from_err().map(|(_, ())| ()))
            } else {
                Either::B(ft_ok(()))
            }
        },
    )
}

// helper functions for build cache when server start.
pub fn build_list(
    conn: SharedConn,
    vec: Vec<u32>,
    key: String,
) -> impl Future<Item = (), Error = ResError> {
    let mut pip = pipe();
    pip.atomic();

    pip.cmd("del").arg(key.as_str()).ignore();

    if vec.len() > 0 {
        pip.cmd("rpush").arg(key.as_str()).arg(vec).ignore();
    }

    pip.query_async(conn).from_err().map(|(_, ())| ())
}

pub fn build_users_cache(
    vec: Vec<User>,
    conn: SharedConn,
) -> impl Future<Item = (), Error = ResError> {
    let mut pip = pipe();
    pip.atomic();
    for v in vec.into_iter() {
        let key = format!("user:{}:set", v.self_id());
        let v: Vec<(&str, String)> = v.into();

        pip.cmd("HMSET")
            .arg(key.as_str())
            .arg(v)
            .ignore()
            .cmd("HMSET")
            .arg(key.as_str())
            .arg(&[("online_status", 0.to_string())])
            .ignore();
    }
    pip.query_async(conn).from_err().map(|(_, ())| ())
}

pub fn build_topics_cache_list(
    is_init: bool,
    vec: Vec<(u32, u32, Option<u32>, NaiveDateTime)>,
    conn: SharedConn,
) -> impl Future<Item = (), Error = ResError> {
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

    pip.query_async(conn).from_err().map(|(_, ())| ())
}

pub fn build_posts_cache_list(
    is_init: bool,
    vec: Vec<(u32, u32, Option<u32>, NaiveDateTime)>,
    conn: SharedConn,
) -> impl Future<Item = (), Error = ResError> {
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

    pipe.query_async(conn).from_err().map(|(_, ())| ())
}

pub fn clear_cache(redis_url: &str) -> Result<(), ResError> {
    let client = redis::Client::open(redis_url).expect("failed to connect to redis server");
    let mut conn = client
        .get_connection()
        .expect("failed to get redis connection");
    Ok(redis::cmd("flushall").query(&mut conn)?)
}
