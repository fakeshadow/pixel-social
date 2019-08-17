use std::{collections::HashMap, time::Duration};

use actix::prelude::{ActorFuture, AsyncContext, Context, Future, WrapFuture};
use chrono::{NaiveDateTime, Utc};
use futures::future::{err as ft_err, join_all, ok as ft_ok, Either};
use redis::{aio::SharedConnection, cmd, pipe, Client};

use crate::model::{
    actors::TalkService,
    category::Category,
    common::{GetSelfId, GetUserId},
    errors::ResError,
    post::Post,
    psn::UserPSNProfile,
    topic::Topic,
    user::User,
};
use crate::{CacheUpdateService, MessageService, PSNService};

// page offsets of list query
const LIMIT: usize = 20;
// use LEX_BASE minus pid and tid before adding to zrange.
const LEX_BASE: u32 = std::u32::MAX;
// list_pop update interval time gap in seconds
const LIST_TIME_GAP: Duration = Duration::from_secs(10);
// hash life is expire time of topic and post hash in seconds.
const HASH_LIFE: usize = 172_800;
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

    pub fn update_user_psn_profile(&self, t: Vec<UserPSNProfile>) {
        actix_rt::spawn(build_hmsets(self.get_conn(), t, "user_psn_profile", false));
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
        page: usize,
        set_key: &'static str,
    ) -> impl Future<Item = (Vec<T>, Vec<u32>), Error = ResError>
    where
        T: std::marker::Send
            + redis::FromRedisValue
            + AttachPermFields<Result = T>
            + GetUserId
            + 'static,
    {
        let start = (page - 1) * 20;
        let end = start + LIMIT - 1;
        self.ids_from_cache_list(list_key, start, end)
            .and_then(move |(conn, ids)| Self::from_cache_with_perm_with_uids(conn, ids, set_key))
    }

    pub fn get_cache_with_uids_from_zrevrange_reverse_lex<T>(
        &self,
        zrange_key: &str,
        page: usize,
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
        page: usize,
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
        page: usize,
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
        page: usize,
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
        self.ids_from_cache_zrange(is_rev, zrange_key, (page - 1) * 20)
            .and_then(move |(conn, mut ids)| {
                if is_reverse_lex {
                    ids = ids.into_iter().map(|i| LEX_BASE - i).collect();
                }
                Self::from_cache_with_perm_with_uids(conn, ids, set_key)
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
        Self::from_cache_with_perm_with_uids(self.get_conn(), ids, set_key)
    }

    pub fn add_topic(&self, t: Topic) {
        let mut pip = pipe();
        pip.atomic();

        let tid = t.id;
        let cid = t.category_id;
        let time = t.created_at.timestamp_millis();
        let key = format!("topic:{}:set", t.self_id());
        let t: Vec<(&str, Vec<u8>)> = t.into();

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
        let p: Vec<(&str, Vec<u8>)> = p.into();

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
        let c: Vec<(&str, Vec<u8>)> = c.into();

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

    pub fn add_psn_request_with_privilege(
        &self,
        req: &str,
    ) -> impl Future<Item = (), Error = ResError> {
        self.add_psn_request(req, 0)
    }

    pub fn add_psn_request_now(&self, req: &str) -> impl Future<Item = (), Error = ResError> {
        self.add_psn_request(req, Utc::now().timestamp_millis())
    }

    fn add_psn_request(&self, req: &str, score: i64) -> impl Future<Item = (), Error = ResError> {
        cmd("ZADD")
            .arg("psn_queue")
            .arg(score)
            .arg(req)
            .query_async(self.get_conn())
            .from_err()
            .map(|(_, ())| ())
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
            .arg(&format!("user:{}:set_perm", uid))
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

pub trait GetQueue
where
    Self: GetSharedConn,
{
    fn get_queue(&self, key: &str) -> Box<dyn Future<Item = String, Error = ResError>> {
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
            pip.query_async(self.get_conn()).from_err().and_then(
                |(_, (mut s, ())): (_, (Vec<String>, _))| s.pop().ok_or(ResError::NoCache),
            ),
        )
    }
}

impl GetQueue for MessageService {}

impl GetQueue for PSNService {}

impl CacheUpdateService {
    pub fn start_interval(&mut self, ctx: &mut Context<Self>) {
        self.update_list_pop(ctx);
    }

    fn update_list_pop(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(LIST_TIME_GAP, move |act, ctx| {
            ctx.spawn(
                act.categories_from_cache()
                    .map_err(|_| ())
                    .into_actor(act)
                    .and_then(|cat, act, _| {
                        let conn = act.get_conn();
                        let yesterday = Utc::now().naive_utc().timestamp_millis() - 86_400_000;
                        let mut vec = Vec::new();

                        for c in cat.iter() {
                            // update_list will also update topic count new.
                            vec.push(Either::A(update_list(Some(c.id), yesterday, conn.clone())));
                            vec.push(Either::B(update_post_count(c.id, yesterday, conn.clone())));
                        }
                        vec.push(Either::A(update_list(None, yesterday, conn)));

                        join_all(vec).map_err(|_| ()).map(|_| ()).into_actor(act)
                    }),
            );
        });
    }
}

pub trait GetSharedConn {
    fn get_conn(&self) -> SharedConnection;
}

impl GetSharedConn for CacheUpdateService {
    fn get_conn(&self) -> SharedConnection {
        self.cache.as_ref().unwrap().clone()
    }
}

impl GetSharedConn for MessageService {
    fn get_conn(&self) -> SharedConnection {
        self.cache.as_ref().unwrap().clone()
    }
}

impl GetSharedConn for PSNService {
    fn get_conn(&self) -> SharedConnection {
        self.cache.as_ref().unwrap().clone()
    }
}

impl GetSharedConn for CacheService {
    fn get_conn(&self) -> SharedConnection {
        self.cache.clone()
    }
}

impl GetSharedConn for TalkService {
    fn get_conn(&self) -> SharedConnection {
        self.cache.clone()
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

pub trait IdsFromList
where
    Self: GetSharedConn,
{
    fn ids_from_cache_list(
        &self,
        list_key: &str,
        start: usize,
        end: usize,
    ) -> Box<dyn Future<Item = (SharedConnection, Vec<u32>), Error = ResError>> {
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
    ) -> Box<dyn Future<Item = (SharedConnection, Vec<u32>), Error = ResError>> {
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

pub trait FromCacheSingle
where
    Self: GetSharedConn,
{
    fn from_cache_single<T>(
        &self,
        key: &[u8],
        set_key: &str,
    ) -> Box<dyn Future<Item = T, Error = ResError>>
    where
        T: std::marker::Send + redis::FromRedisValue + 'static,
    {
        let key = match std::str::from_utf8(key) {
            Ok(k) => k,
            Err(_) => return Box::new(ft_err(ResError::InternalServerError)),
        };

        Box::new(
            cmd("HGETALL")
                .arg(&format!("{}:{}:set", set_key, key))
                .query_async(self.get_conn())
                .from_err()
                .map(|(_, hm)| hm),
        )
    }
}

impl FromCacheSingle for CacheService {}

pub trait FromCache {
    fn from_cache<T>(
        conn: SharedConnection,
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

        Box::new(
            pip.query_async(conn)
                .then(|r: Result<(_, Vec<T>), redis::RedisError>| match r {
                    Ok((_, v)) => {
                        if v.is_empty() {
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

impl FromCache for CacheService {}

impl FromCache for CacheUpdateService {}

impl FromCache for TalkService {}

pub trait FromCacheWithPerm {
    fn from_cache_with_perm<T>(
        conn: SharedConnection,
        ids: Vec<u32>,
        set_key: &str,
    ) -> Box<dyn Future<Item = Vec<(T, HashMap<String, String>)>, Error = ResError>>
    where
        T: std::marker::Send + redis::FromRedisValue + AttachPermFields<Result = T> + 'static,
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
            |r: Result<(_, Vec<(T, _)>), redis::RedisError>| match r {
                Ok((_, v)) => {
                    if v.is_empty() {
                        Err(ResError::IdsFromCache(ids))
                    } else {
                        Ok(v)
                    }
                }
                Err(_) => Err(ResError::IdsFromCache(ids)),
            },
        ))
    }
}

impl FromCacheWithPerm for CacheService {}

impl FromCacheWithPerm for TalkService {}

impl CacheService {
    fn from_cache_with_perm_with_uids<T>(
        conn: SharedConnection,
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
        Self::from_cache_with_perm(conn, ids, set_key).map(|hm: Vec<(T, _)>| {
            let len = hm.len();
            let mut v = Vec::with_capacity(len);
            let mut uids = Vec::with_capacity(len);
            for (t, h) in hm.into_iter() {
                uids.push(t.get_user_id());
                v.push(t.attach_perm_fields(&h));
            }
            (v, uids)
        })
    }
}

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

impl AttachPermFields for User {
    type Result = User;
    fn attach_perm_fields(mut self, h: &HashMap<String, String>) -> Self::Result {
        self.last_online = match h.get("last_online") {
            Some(t) => NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S%.f").ok(),
            None => None,
        };
        self.online_status = match h.get("online_status") {
            Some(s) => s.parse::<u32>().ok(),
            None => None,
        };
        self
    }
}

pub trait UsersFromCache
where
    Self: FromCacheWithPerm + GetSharedConn,
{
    fn users_from_cache(
        &self,
        uids: Vec<u32>,
    ) -> Box<dyn Future<Item = Vec<User>, Error = ResError>> {
        Box::new(
            Self::from_cache_with_perm::<User>(self.get_conn(), uids, "user").map(|hm| {
                let len = hm.len();
                let mut v = Vec::with_capacity(len);
                for (u, h) in hm.into_iter() {
                    v.push(u.attach_perm_fields(&h));
                }
                v
            }),
        )
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
            self.ids_from_cache_list("category_id:meta", 0, 999)
                .and_then(|(conn, vec): (_, Vec<u32>)| Self::from_cache(conn, vec, "category")),
        )
    }
}

impl CategoriesFromCache for CacheService {}

impl CategoriesFromCache for CacheUpdateService {}

pub fn build_hmsets<T>(
    conn: SharedConnection,
    vec: Vec<T>,
    key: &'static str,
    should_expire: bool,
) -> impl Future<Item = (), Error = ()>
where
    T: GetSelfId + Into<Vec<(&'static str, Vec<u8>)>>,
{
    let mut pip = pipe();
    pip.atomic();
    for v in vec.into_iter() {
        let key = format!("{}:{}:set", key, v.self_id());
        let v: Vec<(&str, Vec<u8>)> = v.into();

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
            move |(conn, tids): (SharedConnection, Vec<u32>)| {
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
                pip.query_async(conn).from_err().and_then(
                    |(conn, pids): (SharedConnection, Vec<u32>)| {
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
                    },
                )
            },
        )
    }
}

fn update_post_count(
    cid: u32,
    yesterday: i64,
    conn: SharedConnection,
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
    conn: SharedConnection,
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

            if !vec.is_empty() {
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
    conn: SharedConnection,
    vec: Vec<u32>,
    key: String,
) -> impl Future<Item = (), Error = ResError> {
    let mut pip = pipe();
    pip.atomic();

    pip.cmd("del").arg(key.as_str()).ignore();

    if !vec.is_empty() {
        pip.cmd("rpush").arg(key.as_str()).arg(vec).ignore();
    }

    pip.query_async(conn).from_err().map(|(_, ())| ())
}

pub fn build_users_cache(
    vec: Vec<User>,
    conn: SharedConnection,
) -> impl Future<Item = (), Error = ResError> {
    let mut pip = pipe();
    pip.atomic();
    for v in vec.into_iter() {
        let key = format!("user:{}:set", v.self_id());
        let key_perm = format!("user:{}:set_perm", v.self_id());
        let v: Vec<(&str, Vec<u8>)> = v.into();

        pip.cmd("HMSET")
            .arg(key.as_str())
            .arg(v)
            .ignore()
            .cmd("HMSET")
            .arg(key_perm.as_str())
            .arg(&[("online_status", 0)])
            .ignore();
    }
    pip.query_async(conn).from_err().map(|(_, ())| ())
}

pub fn build_topics_cache_list(
    is_init: bool,
    vec: Vec<(u32, u32, Option<u32>, NaiveDateTime)>,
    conn: SharedConnection,
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
    conn: SharedConnection,
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
