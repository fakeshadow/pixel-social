use std::{
    convert::TryFrom,
    time::Duration,
    collections::HashMap,
};
use futures::future::{ok as ft_ok, Either, join_all};

use actix::prelude::{
    ActorFuture,
    AsyncContext,
    Context,
    Future,
    Handler,
    Message,
    ResponseFuture,
    WrapFuture,
};
use chrono::{Utc, NaiveDateTime};
use redis::{cmd, pipe};

use crate::{
    CacheService,
    CacheUpdateService,
    MessageService,
    TalkService,
};
use crate::model::{
    actors::SharedConn,
    errors::ResError,
    category::Category,
    post::Post,
    topic::Topic,
    user::User,
    common::{GetSelfId, GetUserId},
};

// page offsets of list query
const LIMIT: isize = 20;
// use LEX_BASE minus pid and tid before adding to zrange.
const LEX_BASE: u32 = std::u32::MAX;
// list_pop update interval time gap in seconds
const LIST_TIME_GAP: Duration = Duration::from_secs(5);
// trim list_pop interval time gap
const TRIM_LIST_TIME_GAP: Duration = Duration::from_secs(1800);
// hash life is expire time of topic and post hash in seconds.
const HASH_LIFE: usize = 172800;
// mail life is expire time of mail hash in seconds
const MAIL_LIFE: usize = 3600;

impl CacheService {
    pub fn get_users_cache(&self, uids: Vec<u32>) -> impl Future<Item=Vec<User>, Error=ResError> {
        self.users_from_cache(uids)
    }

    pub fn get_categories_cache(&self) -> impl Future<Item=Vec<Category>, Error=ResError> {
        self.categories_from_cache()
    }

    pub fn get_cache_with_uids_from_list<T>(
        &self,
        list_key: &str,
        page: i64,
        set_key: &'static str,
    ) -> impl Future<Item=(Vec<T>, Vec<u32>), Error=ResError>
        where T: TryFrom<(HashMap<String, String>, HashMap<String, String>), Error=ResError> + GetUserId {
        let start = (page as isize - 1) * 20;
        let end = start + LIMIT - 1;
        self.ids_from_cache_list(list_key, start, end)
            .and_then(move |(conn, ids)|
                Self::hmsets_multi_from_cache(conn, ids, set_key)
                    .and_then(|(h, i)| Self::parse_hashmaps_with_uids(h, i))
            )
    }

    pub fn get_cache_with_uids_from_zrange<T>(
        &self,
        list_key: &str,
        page: i64,
        set_key: &'static str,
    ) -> impl Future<Item=(Vec<T>, Vec<u32>), Error=ResError>
        where T: TryFrom<(HashMap<String, String>, HashMap<String, String>), Error=ResError> + GetUserId {
        self.ids_from_cache_zrange(list_key, ((page - 1) * 20) as usize)
            .and_then(move |(conn, ids)| {
                let ids = ids.into_iter().map(|i| LEX_BASE - i).collect();
                Self::hmsets_multi_from_cache(conn, ids, set_key)
                    .and_then(|(h, i)| Self::parse_hashmaps_with_uids(h, i))
            })
    }
    
    pub fn get_topics_cache_by_ids_with_uids(&self, ids: Vec<u32>) -> impl Future<Item=(Vec<Topic>, Vec<u32>), Error=ResError> {
        Self::hmsets_multi_from_cache(self.get_conn(), ids, "topic")
            .and_then(|(h, i)| Self::parse_hashmaps_with_uids(h, i))
    }

    pub fn get_posts_cache_by_ids_with_uids(&self, ids: Vec<u32>) -> impl Future<Item=(Vec<Post>, Vec<u32>), Error=ResError> {
        Self::hmsets_multi_from_cache(self.get_conn(), ids, "post")
            .and_then(|(h, i)| Self::parse_hashmaps_with_uids(h, i))
    }

    pub fn add_topic_cache(&self, t: Topic) -> impl Future<Item=(), Error=()> {
        let mut pip = pipe();
        pip.atomic();

        let tid = t.id;
        let cid = t.category_id;
        let time = t.created_at.timestamp_millis();
        let key = format!("topic:{}:set", t.self_id());
        let t: Vec<(&str, String)> = t.into();

        pip.cmd("HMSET").arg(key.as_str()).arg(t).ignore()
            .cmd("EXPIRE").arg(key.as_str()).arg(HASH_LIFE).ignore()
            .cmd("HINCRBY").arg(&format!("topic:{}:set_perm", tid)).arg("reply_count").arg(0).ignore()
            .cmd("HINCRBY").arg(&format!("category:{}:set", cid)).arg("topic_count").arg(1).ignore()
            .cmd("lpush").arg(&format!("category:{}:list", cid)).arg(tid).ignore()
            .cmd("ZADD").arg("category:all:topics_time").arg(time).arg(tid).ignore()
            .cmd("ZADD").arg(&format!("category:{}:topics_time", cid)).arg(time).arg(tid).ignore()
            .cmd("ZINCRBY").arg("category:all:topics_reply").arg(0).arg(tid).ignore()
            .cmd("ZINCRBY").arg(&format!("category:{}:topics_reply", cid)).arg(0).arg(tid).ignore();

        pip.query_async(self.get_conn())
            //ToDo: add send report error handling
            .map_err(|_| ())
            .map(|(_, ())| ())
    }

    pub fn add_post_cache(&self, p: Post) -> impl Future<Item=(), Error=()> {
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

        pip.cmd("HMSET").arg(post_key.as_str()).arg(p).ignore()
            .cmd("EXPIRE").arg(post_key.as_str()).arg(HASH_LIFE).ignore()
            .cmd("HINCRBY").arg(&format!("post:{}:set_perm", pid)).arg("reply_count").arg(0).ignore()
            .cmd("HINCRBY").arg(&format!("category:{}:set", cid)).arg("post_count").arg(1).ignore()
            .cmd("lrem").arg(&format!("category:{}:list", cid)).arg(1).arg(tid).ignore()
            .cmd("lpush").arg(&format!("category:{}:list", cid)).arg(tid).ignore()
            .cmd("rpush").arg(&format!("topic:{}:list", tid)).arg(pid).ignore()
            .cmd("HINCRBY").arg(&format!("topic:{}:set_perm", tid)).arg("reply_count").arg(1).ignore()
            .cmd("HSET").arg(&format!("topic:{}:set_perm", tid)).arg("last_reply_time").arg(&time_string).ignore()
            .cmd("ZINCRBY").arg(&format!("topic:{}:posts_reply", tid)).arg(0).arg(LEX_BASE - pid).ignore()
            .cmd("ZADD").arg(&format!("category:{}:topics_time", cid)).arg("XX").arg(time).arg(tid).ignore()
            .cmd("ZADD").arg("category:all:topics_time").arg("XX").arg(time).arg(tid).ignore()
            .cmd("ZINCRBY").arg(&format!("category:{}:topics_reply", cid)).arg(1).arg(tid).ignore()
            .cmd("ZINCRBY").arg("category:all:topics_reply").arg(1).arg(tid).ignore()
            .cmd("ZADD").arg(&format!("category:{}:posts_time", cid)).arg(time).arg(pid).ignore();

        if let Some(pid) = post_id {
            pip.cmd("HSET").arg(&format!("post:{}:set_perm", pid)).arg("last_reply_time").arg(&time_string).ignore()
                .cmd("HINCRBY").arg(&format!("post:{}:set_perm", pid)).arg("reply_count").arg(1).ignore()
                .cmd("ZINCRBY").arg(&format!("topic:{}:posts_reply", tid)).arg(1).arg(LEX_BASE - pid).ignore();
        }

        pip.query_async(self.get_conn())
            .map_err(|_| ())
            .map(|(_, ())| ())
    }
}

impl TalkService {
    pub fn set_online_status(&self, uid: u32, status: u32) -> impl Future<Item=(), Error=ResError> {
        cmd("HMSET")
            .arg(&format!("user:{}:set", uid))
            .arg(&[("online_status", status)])
            .query_async(self.cache.as_ref().unwrap().clone())
            .from_err()
            .map(|(_, ())| ())
    }

    pub fn get_users_cache(&self, uids: Vec<u32>) -> impl Future<Item=Vec<User>, Error=ResError> {
        self.users_from_cache(uids)
    }
}

impl MessageService {
    pub fn get_queue(&self, key: &str) -> impl Future<Item=String, Error=ResError> {
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
            .and_then(|(_, (mut s, ())): (_, (Vec<String>, _))| s
                .pop().ok_or(ResError::NoCache))
    }
}

impl CacheUpdateService {
    pub fn start_interval(&mut self, ctx: &mut Context<Self>) {
        self.update_list_pop(ctx);
        self.trim_list_pop(ctx);
    }

    fn update_list_pop(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(LIST_TIME_GAP, move |act, ctx| {
            let f =
                act.categories_from_cache()
                    .into_actor(act)
                    .map_err(|_, _, _| ())
                    .and_then(|cat, act, _| {
                        let conn = act.get_conn();
                        let yesterday = Utc::now().timestamp_millis() - 86400000;
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
                    });
            ctx.spawn(f);
        });
    }

    fn trim_list_pop(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(TRIM_LIST_TIME_GAP, move |act, ctx| {
            let f =
                act.categories_from_cache()
                    .into_actor(act)
                    .map_err(|_, _, _| ())
                    .and_then(|cat, act, _| {
                        let conn = act.get_conn();
                        let yesterday = Utc::now().timestamp_millis() - 86400000;
                        let mut vec = Vec::new();

                        for c in cat.iter() {
                            vec.push(trim_list(Some(c.id), yesterday, conn.clone()));
                        }
                        vec.push(trim_list(None, yesterday, conn));

                        join_all(vec)
                            .into_actor(act)
                            .map_err(|_, _, _| ())
                            .map(|_, _, _| ())
                    });
            ctx.spawn(f);
        });
    }
}


trait GetSharedConn {
    fn get_conn(&self) -> SharedConn;
}

impl GetSharedConn for TalkService {
    fn get_conn(&self) -> SharedConn { self.cache.as_ref().unwrap().clone() }
}

impl GetSharedConn for CacheService {
    fn get_conn(&self) -> SharedConn {
        self.cache.as_ref().unwrap().clone()
    }
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


trait IdsFromCacheList
    where Self: GetSharedConn {
    fn ids_from_cache_list(&self, list_key: &str, start: isize, end: isize) -> Box<dyn Future<Item=(SharedConn, Vec<u32>), Error=ResError>> {
        Box::new(cmd("lrange")
            .arg(list_key)
            .arg(start)
            .arg(end)
            .query_async(self.get_conn())
            .from_err()
            .and_then(|(conn, ids): (SharedConn, Vec<u32>)| {
                if ids.len() == 0 {
                    Err(ResError::NoContent)
                } else {
                    Ok((conn, ids))
                }
            }))
    }
}

impl IdsFromCacheList for CacheService {}

impl IdsFromCacheList for CacheUpdateService {}

trait IdsFromCacheZrange
    where Self: GetSharedConn {
    fn ids_from_cache_zrange(&self, list_key: &str, offset: usize) -> Box<dyn Future<Item=(SharedConn, Vec<u32>), Error=ResError>> {
        Box::new(cmd("zrevrangebyscore")
            .arg(list_key)
            .arg("+inf")
            .arg("-inf")
            .arg("LIMIT")
            .arg(offset)
            .arg(20)
            .query_async(self.get_conn())
            .from_err()
            .and_then(|(conn, ids): (SharedConn, Vec<u32>)| {
                if ids.len() == 0 {
                    Err(ResError::NoContent)
                } else {
                    Ok((conn, ids))
                }
            }))
    }
}

impl IdsFromCacheZrange for CacheService {}


trait HashMapsFromCache {
    fn hmsets_from_cache(
        conn: SharedConn,
        ids: Vec<u32>,
        set_key: &str,
    ) -> Box<dyn Future<Item=(Vec<HashMap<String, String>>, Vec<u32>), Error=ResError>> {
        let mut pip = pipe();
        pip.atomic();

        for i in ids.iter() {
            pip.cmd("HGETALL").arg(&format!("{}:{}:set", set_key, i));
        }

        Box::new(pip
            .query_async(conn)
            .then(|r| match r {
                Ok((_, hm)) => Ok((hm, ids)),
                Err(_) => Err(ResError::IdsFromCache(ids))
            }))
    }
}

impl HashMapsFromCache for TalkService {}

impl HashMapsFromCache for CacheService {}

impl HashMapsFromCache for CacheUpdateService {}


trait HashMapsTupleFromCache {
    fn hmsets_multi_from_cache(
        conn: SharedConn,
        ids: Vec<u32>,
        set_key: &str,
    ) -> Box<dyn Future<Item=(Vec<(HashMap<String, String>, HashMap<String, String>)>, Vec<u32>), Error=ResError>> {
        let mut pip = pipe();
        pip.atomic();

        for i in ids.iter() {
            pip.cmd("HGETALL").arg(&format!("{}:{}:set", set_key, i))
                .cmd("HGETALL").arg(&format!("{}:{}:set_perm", set_key, i));
        }

        Box::new(pip
            .query_async(conn)
            .then(|r| match r {
                Ok((_, hm)) => Ok((hm, ids)),
                Err(_) => Err(ResError::IdsFromCache(ids))
            }))
    }
}

impl HashMapsTupleFromCache for CacheService {}


trait ParseHashMaps
    where Self: HashMapsFromCache {
    fn parse_hashmaps<T>(hash: Vec<HashMap<String, String>>, ids: Vec<u32>) -> Result<Vec<T>, ResError>
        where T: TryFrom<HashMap<String, String>, Error=ResError> {
        let len = ids.len();
        let mut vec = Vec::with_capacity(len);
        for h in hash.into_iter() {
            if let Some(t) = T::try_from(h).ok() {
                vec.push(t);
            }
        };
        if vec.len() != len {
            return Err(ResError::IdsFromCache(ids));
        };
        Ok(vec)
    }
}

impl ParseHashMaps for CacheService {}

impl ParseHashMaps for CacheUpdateService {}

impl ParseHashMaps for TalkService {}

trait ParseHashMapsWithUids
    where Self: HashMapsFromCache {
    fn parse_hashmaps_with_uids<T>(
        hash: Vec<(HashMap<String, String>, HashMap<String, String>)>,
        ids: Vec<u32>,
    ) -> Result<(Vec<T>, Vec<u32>), ResError>
        where T: TryFrom<(HashMap<String, String>, HashMap<String, String>), Error=ResError> + GetUserId {
        let len = ids.len();
        let mut vec = Vec::with_capacity(len);
        let mut uids = Vec::with_capacity(len);
        for h in hash.into_iter() {
            if let Some(t) = T::try_from(h).ok() {
                uids.push(t.get_user_id());
                vec.push(t);
            }
        };
        if vec.len() != len {
            return Err(ResError::IdsFromCache(ids));
        };
        Ok((vec, uids))
    }
}

impl ParseHashMapsWithUids for CacheService {}


trait UsersFromCache
    where Self: HashMapsFromCache + GetSharedConn + ParseHashMaps {
    fn users_from_cache(&self, uids: Vec<u32>) -> Box<dyn Future<Item=Vec<User>, Error=ResError>> {
        Box::new(Self::hmsets_from_cache(self.get_conn(), uids, "user")
            .and_then(|(h, i)| Self::parse_hashmaps(h, i)))
    }
}

impl UsersFromCache for TalkService {}

impl UsersFromCache for CacheService {}


trait CategoriesFromCache
    where Self: HashMapsFromCache + GetSharedConn + IdsFromCacheList + ParseHashMaps {
    fn categories_from_cache(&self) -> Box<dyn Future<Item=Vec<Category>, Error=ResError>> {
        Box::new(self
            .ids_from_cache_list("category_id:meta", 0, -1)
            .and_then(|(conn, vec): (_, Vec<u32>)|
                Self::hmsets_from_cache(conn, vec, "category")
                    .and_then(|(h, i)| Self::parse_hashmaps(h, i))
            ))
    }
}

impl CategoriesFromCache for CacheService {}

impl CategoriesFromCache for CacheUpdateService {}


impl TryFrom<(HashMap<String, String>, HashMap<String, String>)> for Topic {
    type Error = ResError;
    fn try_from((h, h_p): (HashMap<String, String>, HashMap<String, String>)) -> Result<Self, Self::Error> {
        if h.is_empty() {
            return Err(ResError::DataBaseReadError);
        }
        let last_reply_time = match h_p.get("last_reply_time") {
            Some(t) => NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S%.f").ok(),
            None => None
        };
        let reply_count = match h_p.get("reply_count") {
            Some(t) => t.parse::<u32>().ok(),
            None => None
        };
        Ok(Topic {
            id: h.get("id").ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            user_id: h.get("user_id").ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            category_id: h.get("category_id").ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            title: h.get("title").ok_or(ResError::DataBaseReadError)?.to_owned(),
            body: h.get("body").ok_or(ResError::DataBaseReadError)?.to_owned(),
            thumbnail: h.get("thumbnail").ok_or(ResError::DataBaseReadError)?.to_owned(),
            created_at: NaiveDateTime::parse_from_str(h.get("created_at").ok_or(ResError::DataBaseReadError)?, "%Y-%m-%d %H:%M:%S%.f")?,
            updated_at: NaiveDateTime::parse_from_str(h.get("updated_at").ok_or(ResError::DataBaseReadError)?, "%Y-%m-%d %H:%M:%S%.f")?,
            last_reply_time,
            is_locked: h.get("is_locked").ok_or(ResError::DataBaseReadError)?.parse::<bool>().map_err(|_| ResError::ParseError)?,
            reply_count,
        })
    }
}

impl TryFrom<(HashMap<String, String>, HashMap<String, String>)> for Post {
    type Error = ResError;
    fn try_from((h, h_p): (HashMap<String, String>, HashMap<String, String>)) -> Result<Self, Self::Error> {
        if h.is_empty() {
            return Err(ResError::DataBaseReadError);
        }
        let post_id = match h.get("post_id").ok_or(ResError::DataBaseReadError)?.parse::<u32>().ok() {
            Some(id) => if id == 0 { None } else { Some(id) },
            None => None,
        };
        let last_reply_time = match h_p.get("last_reply_time") {
            Some(t) => NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S%.f").ok(),
            None => None
        };
        let reply_count = match h_p.get("reply_count") {
            Some(t) => t.parse::<u32>().ok(),
            None => None
        };
        Ok(Post {
            id: h.get("id").ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            user_id: h.get("user_id").ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            topic_id: h.get("topic_id").ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            category_id: h.get("category_id").ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            post_id,
            post_content: h.get("post_content").ok_or(ResError::DataBaseReadError)?.to_owned(),
            created_at: NaiveDateTime::parse_from_str(h.get("created_at").ok_or(ResError::DataBaseReadError)?, "%Y-%m-%d %H:%M:%S%.f")?,
            updated_at: NaiveDateTime::parse_from_str(h.get("updated_at").ok_or(ResError::DataBaseReadError)?, "%Y-%m-%d %H:%M:%S%.f")?,
            last_reply_time,
            is_locked: h.get("is_locked").ok_or(ResError::DataBaseReadError)?.parse::<bool>().map_err(|_| ResError::ParseError)?,
            reply_count,
        })
    }
}

impl TryFrom<HashMap<String, String>> for User {
    type Error = ResError;
    fn try_from(h: HashMap<String, String>) -> Result<Self, Self::Error> {
        if h.is_empty() {
            return Err(ResError::DataBaseReadError);
        }
        Ok(User {
            id: h.get("id").ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            username: h.get("username").ok_or(ResError::DataBaseReadError)?.to_owned(),
            email: h.get("email").ok_or(ResError::DataBaseReadError)?.to_owned(),
            hashed_password: "1".to_owned(),
            avatar_url: h.get("avatar_url").ok_or(ResError::DataBaseReadError)?.to_owned(),
            signature: h.get("signature").ok_or(ResError::DataBaseReadError)?.to_owned(),
            created_at: NaiveDateTime::parse_from_str(h.get("created_at").ok_or(ResError::DataBaseReadError)?, "%Y-%m-%d %H:%M:%S%.f")?,
            privilege: h.get("privilege").ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            show_email: h.get("show_email").ok_or(ResError::DataBaseReadError)?.parse::<bool>().map_err(|_| ResError::ParseError)?,
            online_status: h.get("online_status").ok_or(ResError::DataBaseReadError)?.parse::<u32>().ok(),
            last_online: NaiveDateTime::parse_from_str(h.get("created_at").ok_or(ResError::DataBaseReadError)?, "%Y-%m-%d %H:%M:%S%.f").ok(),
        })
    }
}

impl TryFrom<HashMap<String, String>> for Category {
    type Error = ResError;
    fn try_from(h: HashMap<String, String>) -> Result<Self, Self::Error> {
        if h.is_empty() {
            return Err(ResError::DataBaseReadError);
        }
        Ok(Category {
            id: h.get("id").ok_or(ResError::DataBaseReadError)?.parse::<u32>()?,
            name: h.get("name").ok_or(ResError::DataBaseReadError)?.to_owned(),
            thumbnail: h.get("thumbnail").ok_or(ResError::DataBaseReadError)?.to_owned(),
            topic_count: h.get("topic_count").ok_or(ResError::DataBaseReadError)?.parse::<u32>().ok(),
            post_count: h.get("post_count").ok_or(ResError::DataBaseReadError)?.parse::<u32>().ok(),
            topic_count_new: h.get("topic_count_new").ok_or(ResError::DataBaseReadError)?.parse::<u32>().ok(),
            post_count_new: h.get("post_count_new").ok_or(ResError::DataBaseReadError)?.parse::<u32>().ok(),
        })
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
            ("show_email", self.show_email.to_string())
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
            ("is_locked", self.is_locked.to_string())
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
            ("is_locked", self.is_locked.to_string())
        ]
    }
}

impl Into<Vec<(&str, String)>> for Category {
    fn into(self) -> Vec<(&'static str, String)> {
        vec![
            ("id", self.id.to_string()),
            ("name", self.name.to_owned()),
            ("thumbnail", self.thumbnail.to_owned())
        ]
    }
}

#[derive(Message)]
pub struct AddActivationMail(pub User);

impl Handler<AddActivationMail> for CacheService {
    type Result = ();

    fn handle(&mut self, msg: AddActivationMail, ctx: &mut Self::Context) {
        let user = msg.0;
        let uuid = uuid::Uuid::new_v4().to_string();
        let mail = crate::model::messenger::Mail::new_activation(user.email.as_str(), uuid.as_str());

        if let Some(s) = serde_json::to_string(&mail).ok() {
            let mut pip = pipe();
            pip.atomic();
            pip.cmd("ZADD").arg("mail_queue").arg(user.id).arg(s.as_str()).ignore()
                .cmd("HSET").arg(uuid.as_str()).arg("user_id").arg(user.id).ignore()
                .cmd("EXPIRE").arg(uuid.as_str()).arg(MAIL_LIFE).ignore();

            ctx.spawn(pip
                .query_async(self.get_conn())
                .into_actor(self)
                .map_err(|_, _, _| ())
                .map(|(_, ()), _, _| ()));
        }
    }
}

#[derive(Message)]
pub struct AddedCategory(pub Category);

#[derive(Message)]
pub enum UpdateCache<T> {
    Topic(Vec<T>),
    Post(Vec<T>),
    User(Vec<T>),
    Category(Vec<T>),
}

#[derive(Message)]
pub enum DeleteCache {
    Mail(String),
}


pub struct RemoveCategoryCache(pub u32);

pub struct ActivateUser(pub String);

impl Message for ActivateUser {
    type Result = Result<u32, ResError>;
}


impl Message for RemoveCategoryCache {
    type Result = Result<(), ResError>;
}

impl<T> Handler<UpdateCache<T>> for CacheService
    where T: GetSelfId + Into<Vec<(&'static str, String)>> + 'static {
    type Result = ();

    fn handle(&mut self, msg: UpdateCache<T>, ctx: &mut Self::Context) -> Self::Result {
        let conn = self.get_conn();

        let f = match msg {
            UpdateCache::Topic(vec) => build_hmsets(conn, vec, "topic", true),
            UpdateCache::Post(vec) => build_hmsets(conn, vec, "post", true),
            UpdateCache::User(vec) => build_hmsets(conn, vec, "user", false),
            UpdateCache::Category(vec) => build_hmsets(conn, vec, "category", false)
        };

        ctx.spawn(f
            .into_actor(self)
            .map_err(|_, _, _| ())
            .map(|_, _, _| ()));
    }
}

impl Handler<AddedCategory> for CacheService {
    type Result = ();

    fn handle(&mut self, msg: AddedCategory, ctx: &mut Self::Context) -> Self::Result {
        let mut pip = pipe();
        pip.atomic();

        let id = msg.0.id;
        let c: Vec<(&str, String)> = msg.0.into();

        pip.cmd("rpush").arg("category_id:meta").arg(id).ignore()
            .cmd("HMSET").arg(&format!("category:{}:set", id)).arg(c).ignore();
        let f = pip
            .query_async(self.get_conn())
            .into_actor(self)
            .map_err(|_, _, _| ())
            .map(|(_, ()), _, _| ());

        ctx.spawn(f);
    }
}

//ToDo: move this handler to delete cache enum
impl Handler<RemoveCategoryCache> for CacheService {
    type Result = ResponseFuture<(), ResError>;

    fn handle(&mut self, msg: RemoveCategoryCache, _: &mut Self::Context) -> Self::Result {
        let key = format!("category:{}:set", msg.0);
        let fields = ["id", "name", "topic_count", "post_count", "subscriber_count", "thumbnail"];

        let mut pipe = pipe();
        pipe.atomic();
        pipe.cmd("lrem").arg(1).arg("category_id:meta");

        for f in fields.to_vec() {
            pipe.cmd("hdel").arg(&key).arg(f);
        }

        Box::new(pipe.query_async(self.get_conn())
            .from_err()
            .map(|(_, _): (_, usize)| ()))
    }
}

impl Handler<DeleteCache> for CacheService {
    type Result = ();

    fn handle(&mut self, msg: DeleteCache, ctx: &mut Self::Context) -> Self::Result {
        let (key, fields) = match msg {
            DeleteCache::Mail(uuid) => {
                let fields = ["user_id", "uuid"];
                (uuid, fields)
            }
        };

        let mut pip = pipe();
        pip.atomic();

        for f in fields.to_vec() {
            pip.cmd("hdel").arg(&key).arg(f);
        }

        ctx.spawn(pip
            .query_async(self.get_conn())
            .into_actor(self)
            .map_err(|_, _, _| ())
            .map(|(_, _): (_, usize), _, _| ()));
    }
}

impl Handler<ActivateUser> for CacheService {
    type Result = ResponseFuture<u32, ResError>;

    fn handle(&mut self, msg: ActivateUser, _: &mut Self::Context) -> Self::Result {
        let f = cmd("HGETALL")
            .arg(&msg.0)
            .query_async(self.get_conn())
            .from_err()
            .and_then(move |(_, hm): (_, HashMap<String, String>)| {
                Ok(hm.get("user_id").map(String::as_str).ok_or(ResError::Unauthorized)?.parse::<u32>()?)
            });
        Box::new(f)
    }
}

fn update_post_count(
    cid: u32,
    yesterday: i64,
    conn: SharedConn,
) -> impl Future<Item=(), Error=ResError> {
    let time_key = format!("category:{}:posts_time", cid);
    let set_key = format!("category:{}:set", cid);

    cmd("ZCOUNT")
        .arg(&time_key)
        .arg(yesterday)
        .arg("+inf")
        .query_async(conn)
        .from_err()
        .and_then(move |(conn, count): (_, u32)| {
            cmd("HMSET")
                .arg(&set_key)
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
) -> impl Future<Item=(), Error=ResError> {
    let (list_key, time_key, reply_key, set_key) = match cid.as_ref() {
        Some(cid) => (
            format!("category:{}:list_pop", cid),
            format!("category:{}:topics_time", cid),
            format!("category:{}:topics_reply", cid),
            Some(format!("category:{}:set", cid))),
        None => (
            "category:all:list_pop".to_owned(),
            "category:all:topics_time".to_owned(),
            "category:all:topics_reply".to_owned(),
            None
        )
    };

    let mut pip = pipe();
    pip.atomic();
    pip.cmd("ZREVRANGEBYSCORE")
        .arg(&time_key)
        .arg("+inf")
        .arg(yesterday)
        .arg("WITHSCORES")
        .cmd("ZREVRANGEBYSCORE")
        .arg(&reply_key)
        .arg("+inf")
        .arg("-inf")
        .arg("WITHSCORES");

    pip.query_async(conn)
        .from_err()
        .and_then(move |(conn, (tids, mut counts)): (_, (HashMap<u32, i64>, Vec<(u32, u32)>))| {
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
                Either::A(pip
                    .query_async(conn)
                    .from_err()
                    .map(|(_, ())| ()))
            } else {
                Either::B(ft_ok(()))
            }
        })
}

fn trim_list(
    cid: Option<u32>,
    yesterday: i64,
    conn: SharedConn,
) -> impl Future<Item=(), Error=ResError> {
    let (time_key, reply_key, time_key_post) = match cid.as_ref() {
        Some(cid) => (
            format!("category:{}:topics_time", cid),
            format!("category:{}:topics_reply", cid),
            Some(format!("category:{}:posts_time", cid))),
        None => (
            "category:all:topics_time".to_owned(),
            "category:all:topics_reply".to_owned(),
            None
        )
    };

    let mut pip = pipe();
    pip.atomic();
    pip.cmd("zrevrangebyscore")
        .arg(&time_key)
        .arg("+inf")
        .arg(yesterday)
        .cmd("ZREVRANGEBYSCORE")
        .arg(&reply_key)
        .arg("+inf")
        .arg("-inf")
        .arg("WITHSCORES");

    pip.query_async(conn)
        .from_err()
        .and_then(move |(conn, (tids, counts)): (_, (Vec<u32>, Vec<(u32, u32)>))| {
            let mut pipe = pipe();
            for (tid, _) in counts.into_iter() {
                if !tids.contains(&tid) {
                    pipe.cmd("zrem").arg(&reply_key).arg(tid).ignore();
                }
            }
            pipe.cmd("ZREMRANGEBYSCORE").arg(&time_key).arg(yesterday).arg("-inf").ignore();

            if let Some(key) = time_key_post {
                pipe.cmd("ZREMRANGEBYSCORE").arg(&key).arg(yesterday).arg("-inf").ignore();
            }

            pipe.query_async(conn)
                .from_err()
                .map(|(_, ())| ())
        })
}

// helper functions
pub fn build_hmsets<T>(
    conn: SharedConn,
    vec: Vec<T>,
    key: &'static str,
    should_expire: bool,
) -> impl Future<Item=(), Error=ResError>
    where T: GetSelfId + Into<Vec<(&'static str, String)>> {
    let mut pip = pipe();
    pip.atomic();
    for v in vec.into_iter() {
        let key = format!("{}:{}:set", key, v.self_id());
        pip.cmd("HMSET")
            .arg(key.as_str())
            .arg(v.into())
            .ignore();
        if should_expire {
            pip.cmd("expire")
                .arg(key.as_str())
                .arg(HASH_LIFE)
                .ignore();
        }
    }
    pip.query_async(conn)
        .from_err()
        .map(|(_, ())| ())
}

pub fn build_list(
    conn: SharedConn,
    vec: Vec<u32>,
// pass lpush or rpush as cmd
    cmd: &'static str,
    key: String,
) -> impl Future<Item=(), Error=ResError> {
    let mut pip = pipe();
    pip.atomic();

    pip.cmd("del")
        .arg(key.as_str())
        .ignore();

    if vec.len() > 0 {
        pip.cmd(cmd)
            .arg(key.as_str())
            .arg(vec)
            .ignore();
    }

    pip.query_async(conn)
        .from_err()
        .map(|(_, ())| ())
}


pub fn build_users_cache(
    vec: Vec<User>,
    conn: SharedConn,
) -> impl Future<Item=(), Error=ResError> {
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
    pip.query_async(conn)
        .from_err()
        .map(|(_, ())| ())
}

// startup helper fn
pub fn build_topics_cache_list(
    is_init: bool,
    vec: Vec<(u32, u32, u32, NaiveDateTime)>,
    conn: SharedConn,
) -> impl Future<Item=(), Error=ResError> {
    let mut pip = pipe();
    pip.atomic();

    for (tid, cid, count, time) in vec.into_iter() {
        if is_init {
            let time = time.timestamp_millis();
            // ToDo: query existing cache for topic's real last reply time.
            pip.cmd("ZADD").arg("category:all:topics_time").arg(time).arg(tid).ignore()
                .cmd("ZADD").arg("category:all:topics_reply").arg(count).arg(tid).ignore()
                .cmd("ZADD").arg(&format!("category:{}:topics_time", cid)).arg(time).arg(tid).ignore()
                .cmd("ZADD").arg(&format!("category:{}:topics_reply", cid)).arg(count).arg(tid).ignore();
        }
        // set topic's reply count to perm key that never expire.
        pip.cmd("HSET").arg(&format!("topic:{}:set_perm", tid)).arg("reply_count").arg(count).ignore();
    }

    pip.query_async(conn)
        .from_err()
        .map(|(_, ())| ())
}

pub fn build_posts_cache_list(
    vec: Vec<(u32, u32, u32)>,
    conn: SharedConn,
) -> impl Future<Item=(), Error=ResError> {
    let mut pipe = pipe();
    pipe.atomic();

    for (tid, pid, count) in vec.into_iter() {
        pipe.cmd("ZADD").arg(&format!("topic:{}:posts_reply", tid)).arg(count).arg(LEX_BASE - pid).ignore()
            // set post's reply count to perm key that never expire.
            .cmd("HSET").arg(&format!("post:{}:set_perm", pid)).arg("reply_count").arg(count).ignore();
    }

    pipe.query_async(conn)
        .from_err()
        .map(|(_, ())| ())
}

pub fn clear_cache(redis_url: &str) -> Result<(), ResError> {
    let client = redis::Client::open(redis_url).expect("failed to connect to redis server");
    let mut conn = client.get_connection().expect("failed to get redis connection");
    Ok(redis::cmd("flushall").query(&mut conn)?)
}
