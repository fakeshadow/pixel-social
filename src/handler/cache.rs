use std::cell::RefCell;

use actix::prelude::{Future, Recipient};
use chrono::{NaiveDateTime, Utc};
use futures::future::{ok as ft_ok, Either};
use futures::IntoFuture;
use redis::{aio::SharedConnection, cmd, pipe, Client};

use crate::handler::cache_update::CacheFailedMessage;
use crate::model::{
    actors::TalkService,
    cache_schema::{HashMapBrown, RefTo},
    category::Category,
    common::{SelfId, SelfIdString, SelfUserId},
    errors::ResError,
    post::Post,
    topic::Topic,
    user::User,
};
use crate::PSNService;

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

impl CacheService {
    pub fn init(
        redis_url: &str,
        recipient: Recipient<CacheFailedMessage>,
    ) -> impl Future<Item = CacheService, Error = ()> {
        let url = redis_url.to_owned();
        Client::open(redis_url)
            .unwrap_or_else(|e| panic!("{:?}", e))
            .get_shared_async_connection()
            .map_err(|e| panic!("{:?}", e))
            .map(|c| CacheService {
                url,
                cache: RefCell::new(c),
                recipient,
            })
    }
}

impl CacheService {
    pub fn update_users(&self, u: &[User]) {
        actix::spawn(build_hmsets(self.get_conn(), u, USER_U8, false).map_err(|_| ()));
    }

    pub fn update_categories(&self, c: &[Category]) {
        actix::spawn(build_hmsets(self.get_conn(), c, CATEGORY_U8, false).map_err(|_| ()));
    }

    pub fn update_topics(&self, t: &[Topic]) {
        actix::spawn(build_hmsets(self.get_conn(), t, TOPIC_U8, true).map_err(|_| ()));
    }

    pub fn update_posts(&self, t: &[Post]) {
        actix::spawn(build_hmsets(self.get_conn(), t, POST_U8, true).map_err(|_| ()));
    }

    pub fn get_cache_with_uids_from_list<T>(
        &self,
        list_key: &str,
        page: usize,
        set_key: &'static [u8],
    ) -> impl Future<Item = (Vec<T>, Vec<u32>), Error = ResError>
    where
        T: std::marker::Send + redis::FromRedisValue + SelfUserId + 'static,
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
        set_key: &'static [u8],
    ) -> impl Future<Item = (Vec<T>, Vec<u32>), Error = ResError>
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
    ) -> impl Future<Item = (Vec<T>, Vec<u32>), Error = ResError>
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
    ) -> impl Future<Item = (Vec<T>, Vec<u32>), Error = ResError>
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
    ) -> impl Future<Item = (Vec<T>, Vec<u32>), Error = ResError>
    where
        T: std::marker::Send + redis::FromRedisValue + SelfUserId + 'static,
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
        set_key: &'static [u8],
    ) -> impl Future<Item = (Vec<T>, Vec<u32>), Error = ResError>
    where
        T: std::marker::Send + redis::FromRedisValue + SelfUserId + 'static,
    {
        Self::from_cache_with_perm_with_uids(self.get_conn(), ids, set_key)
    }

    // return topic and post as error if for whatever reason the insert to redis is failed.
    // the return topic/post will be send to CacheUpdateService actor and retry until they are all successfully insert into redis.
    pub fn add_topic(&self, t: Topic) -> impl Future<Item = (), Error = Topic> {
        self.add_topic_cache(&t).map_err(|_| t)
    }

    pub fn add_post(&self, p: Post) -> impl Future<Item = (), Error = Post> {
        self.add_post_cache(&p).map_err(|_| p)
    }

    // Don't confused these with update_topics/posts/users methods. The latter methods run in spawned futures and the errors are ignored.
    // They are separate methods as we don't want to retry every failed update cache for most times the data are from expired cache query and not actual content update.
    pub fn update_topics_return_fail(
        &self,
        t: Vec<Topic>,
    ) -> impl Future<Item = (), Error = Vec<Topic>> {
        build_hmsets(self.get_conn(), &t, TOPIC_U8, true).map_err(|_| t)
    }

    pub fn update_post_return_fail(
        &self,
        p: Vec<Post>,
    ) -> impl Future<Item = (), Error = Vec<Post>> {
        build_hmsets(self.get_conn(), &p, POST_U8, true).map_err(|_| p)
    }

    pub fn update_user_return_fail(
        &self,
        u: Vec<User>,
    ) -> impl Future<Item = (), Error = Vec<User>> {
        build_hmsets(self.get_conn(), &u, USER_U8, false).map_err(|_| u)
    }

    // only send message once and ignore the result.
    pub fn add_fail_topic(&self, t: Topic) {
        let _ = self.recipient.do_send(CacheFailedMessage::FailedTopic(t));
    }

    pub fn add_fail_post(&self, p: Post) {
        let _ = self.recipient.do_send(CacheFailedMessage::FailedPost(p));
    }

    pub fn add_fail_topic_update(&self, t: Vec<Topic>) {
        let _ = self
            .recipient
            .do_send(CacheFailedMessage::FailedTopicUpdate(t));
    }
    pub fn add_fail_post_update(&self, p: Vec<Post>) {
        let _ = self
            .recipient
            .do_send(CacheFailedMessage::FailedPostUpdate(p));
    }
    pub fn add_fail_user_update(&self, u: Vec<User>) {
        let _ = self.recipient.do_send(CacheFailedMessage::FailedUser(u));
    }

    pub fn add_category(&self, c: &Category) {
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

        actix::spawn(
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

impl GetQueue for PSNService {}

pub trait GetSharedConn {
    fn get_conn(&self) -> SharedConnection;
}

impl GetSharedConn for PSNService {
    fn get_conn(&self) -> SharedConnection {
        self.cache.as_ref().unwrap().borrow().clone()
    }
}

impl GetSharedConn for CacheService {
    fn get_conn(&self) -> SharedConnection {
        self.cache.borrow().clone()
    }
}

impl GetSharedConn for TalkService {
    fn get_conn(&self) -> SharedConnection {
        self.cache.clone()
    }
}

/// redis connection is only checked on insert request.
/// Connections are not shared between threads so the recovery will happen separately.
pub trait CheckCacheConn
where
    Self: GetSharedConn,
{
    fn check_cache_conn(
        &self,
    ) -> Box<dyn Future<Item = Option<SharedConnection>, Error = ResError>> {
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
                .from_err(),
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

impl CheckCacheConn for PSNService {
    fn self_url(&self) -> String {
        self.cache_url.to_owned()
    }

    fn replace_cache(&self, c: SharedConnection) {
        self.cache.as_ref().map(|s| s.replace(c));
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

/// ids from list and sorted set will return an error if the result is empty.
/// we assume no data can be found on database if we don't have according id in cache.
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

pub trait HashMapBrownFromCache
where
    Self: GetSharedConn,
{
    fn hash_map_brown_from_cache(
        &self,
        key: &str,
    ) -> Box<dyn Future<Item = HashMapBrown<String, String>, Error = ResError>> {
        Box::new(
            cmd("HGETALL")
                .arg(key)
                .query_async(self.get_conn())
                .from_err()
                .map(|(_, hm)| hm),
        )
    }
}

impl HashMapBrownFromCache for CacheService {}

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
        key: &str,
        set_key: &str,
    ) -> Box<dyn Future<Item = T, Error = ResError>>
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

impl FromCacheSingle for CacheService {}

pub trait FromCache {
    fn from_cache<T>(
        conn: SharedConnection,
        ids: Vec<u32>,
        set_key: &'static [u8],
        have_perm_fields: bool,
        // return input ids so the following function can also include the ids when mapping error to ResError::IdsFromCache.
    ) -> Box<dyn Future<Item = Vec<T>, Error = ResError>>
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

impl FromCache for CacheService {}

impl FromCache for TalkService {}

impl CacheService {
    fn from_cache_with_perm_with_uids<T>(
        conn: SharedConnection,
        ids: Vec<u32>,
        set_key: &'static [u8],
    ) -> impl Future<Item = (Vec<T>, Vec<u32>), Error = ResError>
    where
        T: std::marker::Send + redis::FromRedisValue + SelfUserId + 'static,
    {
        Self::from_cache(conn, ids, set_key, true).map(|t: Vec<T>| {
            let len = t.len();
            let mut uids = Vec::with_capacity(len);
            for t in t.iter() {
                uids.push(t.get_user_id());
            }
            (t, uids)
        })
    }
}

pub trait UsersFromCache
where
    Self: GetSharedConn + FromCache,
{
    fn users_from_cache(
        &self,
        uids: Vec<u32>,
    ) -> Box<dyn Future<Item = Vec<User>, Error = ResError>> {
        Box::new(Self::from_cache::<User>(
            self.get_conn(),
            uids,
            USER_U8,
            true,
        ))
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
                .and_then(|(conn, vec): (_, Vec<u32>)| {
                    Self::from_cache(conn, vec, CATEGORY_U8, false)
                }),
        )
    }
}

impl CategoriesFromCache for CacheService {}

pub trait AddTopicCache: GetSharedConn {
    fn add_topic_cache(&self, t: &Topic) -> Box<dyn Future<Item = (), Error = ResError>> {
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
                //ToDo: add error handling
                .from_err()
                .map(|(_, ())| ()),
        )
    }
}

impl AddTopicCache for CacheService {}

pub trait AddPostCache: GetSharedConn {
    fn add_post_cache(&self, p: &Post) -> Box<dyn Future<Item = (), Error = ResError>> {
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
}

impl AddPostCache for CacheService {}

pub trait BulkUpdateCache: GetSharedConn {
    fn bulk_add_update_cache(
        &self,
        t: Vec<&Topic>,
        p: Vec<&Post>,
        u: Vec<&User>,
    ) -> Box<dyn Future<Item = (), Error = ResError>> {
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

        Box::new(
            pip.query_async(self.get_conn())
                .from_err()
                .map(|(_, ())| println!("updating cache")),
        )
    }
}

pub fn build_hmsets<T>(
    conn: SharedConnection,
    vec: &[T],
    set_key: &'static [u8],
    should_expire: bool,
) -> impl Future<Item = (), Error = ResError>
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
        .from_err()
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

pub fn update_post_count(
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
            if count > 0 {
                Either::A(
                    cmd("HMSET")
                        .arg(set_key.as_str())
                        .arg(&[("post_count_new", count)])
                        .query_async(conn)
                        .from_err()
                        .map(|(_, ())| ()),
                )
            } else {
                Either::B(ft_ok(()))
            }
        })
}

type ListWithSortedRange = (HashMapBrown<u32, i64>, Vec<(u32, u32)>);

pub fn update_list(
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
