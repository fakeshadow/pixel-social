use chrono::NaiveDateTime;
use redis::{from_redis_value, ErrorKind, FromRedisValue, RedisResult, Value};

use crate::model::{category::Category, post::Post, psn::UserPSNProfile, topic::Topic, user::User};

// any from redis value error will lead to a database query.
// (except the data that only live in redis.They are ignored if for whatever reason they are lost or can't be load.)
// Cache failure could potential be fixed after that.

// trait to handle pipelined response where every X:X:set key followed by it's X:X:set_perm key.
// take in a function to pattern match the X:X:set_perm key's fields.
trait CrateFromRedisValues
where
    Self: Sized + Default + FromRedisValue,
{
    fn crate_from_redis_values<F>(
        items: &[Value],
        mut attach_perm_fields: F,
    ) -> RedisResult<Vec<Self>>
    where
        F: FnMut(&mut Self, &[u8], &[u8]) + Sized,
    {
        if items.is_empty() {
            return Err((ErrorKind::ResponseError, "Response is empty").into());
        }

        let len = items.len();

        let mut vec = Vec::with_capacity(len);

        let mut i = 0usize;

        loop {
            if i >= len {
                break;
            }
            let mut t: Self = FromRedisValue::from_redis_value(&items[i])?;

            if let Some(v) = items.get(i + 1) {
                if let Value::Bulk(ref items) = *v {
                    let mut iter = items.iter();

                    while let Some(k) = iter.next() {
                        if let Some(v) = iter.next() {
                            if let Ok(k) = from_redis_value::<Vec<u8>>(k) {
                                if let Ok(v) = from_redis_value::<Vec<u8>>(v) {
                                    attach_perm_fields(&mut t, k.as_slice(), v.as_slice())
                                }
                            }
                        }
                    }
                }
            }
            vec.push(t);
            i += 2;
        }
        Ok(vec)
    }
}

impl CrateFromRedisValues for Topic {}

impl CrateFromRedisValues for Post {}

impl CrateFromRedisValues for User {}

// trait to handle single key response with multiple fields.
// take in a function to pattern match each field
trait CrateFromRedisValue {
    fn crate_from_redis_value<F>(v: &Value, mut parse_pattern: F) -> RedisResult<Self>
    where
        Self: Default + std::fmt::Debug,
        F: FnMut(&mut Self, &[u8], &Value) -> RedisResult<()> + Sized,
    {
        match *v {
            Value::Bulk(ref items) => {
                if items.is_empty() {
                    return Err((ErrorKind::ResponseError, "Response is empty").into());
                }

                let mut t = Self::default();
                let mut iter = items.iter();

                while let Some(k) = iter.next() {
                    if let Some(v) = iter.next() {
                        let key: Vec<u8> = from_redis_value(k)?;
                        if let Err(e) = parse_pattern(&mut t, key.as_slice(), v) {
                            return Err(e);
                        }
                    }
                }
                Ok(t)
            }
            _ => Err((ErrorKind::ResponseError, "Response type not compatible").into()),
        }
    }
}

impl CrateFromRedisValue for Topic {}

impl CrateFromRedisValue for Post {}

impl CrateFromRedisValue for User {}

impl CrateFromRedisValue for Category {}

impl CrateFromRedisValue for UserPSNProfile {}

impl FromRedisValue for Topic {
    fn from_redis_value(v: &Value) -> RedisResult<Topic> {
        Topic::crate_from_redis_value(v, |t, k, v| {
            match k {
                b"topic" => {
                    let s = from_redis_value::<Vec<u8>>(v)?;
                    let tt = serde_json::from_slice::<Topic>(&s)
                        .map_err(|_| (ErrorKind::ResponseError, "Response type not compatible"))?;
                    t.id = tt.id;
                    t.user_id = tt.user_id;
                    t.category_id = tt.category_id;
                    t.title = tt.title;
                    t.body = tt.body;
                    t.thumbnail = tt.thumbnail;
                    t.created_at = tt.created_at;
                    t.updated_at = tt.updated_at;
                    t.is_locked = tt.is_locked;
                    t.is_visible = tt.is_visible;
                }
                _ => return Err((ErrorKind::ResponseError, "Response type not compatible").into()),
            };
            Ok(())
        })
    }

    fn from_redis_values(items: &[Value]) -> RedisResult<Vec<Topic>> {
        Topic::crate_from_redis_values(items, |t, k, v| match k {
            b"last_reply_time" => t.last_reply_time = parse_naive_date_time(&v),
            b"reply_count" => t.reply_count = parse_count(&v),
            _ => {}
        })
    }
}

impl FromRedisValue for Post {
    fn from_redis_value(v: &Value) -> RedisResult<Post> {
        Post::crate_from_redis_value(v, |p, k, v| {
            match k {
                b"post" => {
                    let v = from_redis_value::<Vec<u8>>(v)?;
                    let pp = serde_json::from_slice::<Post>(&v)
                        .map_err(|_| (ErrorKind::ResponseError, "Response type not compatible"))?;
                    p.id = pp.id;
                    p.user_id = pp.user_id;
                    p.topic_id = pp.topic_id;
                    p.category_id = pp.category_id;
                    p.post_id = pp.post_id;
                    p.post_content = pp.post_content;
                    p.created_at = pp.created_at;
                    p.updated_at = pp.updated_at;
                    p.is_locked = pp.is_locked;
                }
                _ => return Err((ErrorKind::ResponseError, "Response type not compatible").into()),
            };
            Ok(())
        })
    }

    fn from_redis_values(items: &[Value]) -> RedisResult<Vec<Post>> {
        Post::crate_from_redis_values(items, |p, k, v| match k {
            b"last_reply_time" => p.last_reply_time = parse_naive_date_time(&v),
            b"reply_count" => p.reply_count = parse_count(&v),
            _ => {}
        })
    }
}

impl FromRedisValue for User {
    fn from_redis_value(v: &Value) -> RedisResult<User> {
        User::crate_from_redis_value(v, |u, k, v| {
            match k {
                b"user" => {
                    let v = from_redis_value::<Vec<u8>>(v)?;
                    let uu = serde_json::from_slice::<User>(&v)
                        .map_err(|_| (ErrorKind::ResponseError, "Response type not compatible"))?;
                    u.id = uu.id;
                    u.username = uu.username;
                    u.email = uu.email;
                    u.avatar_url = uu.avatar_url;
                    u.signature = uu.signature;
                    u.created_at = uu.created_at;
                    u.privilege = uu.privilege;
                    u.show_email = uu.show_email;
                }
                _ => return Err((ErrorKind::ResponseError, "Response type not compatible").into()),
            };
            Ok(())
        })
    }

    fn from_redis_values(items: &[Value]) -> RedisResult<Vec<User>> {
        User::crate_from_redis_values(items, |u, k, v| match k {
            b"last_online" => u.last_online = parse_naive_date_time(&v),
            b"online_status" => u.online_status = parse_count(&v),
            _ => {}
        })
    }
}

// from_redis_values is ignored as Category key doesn't have any perm fields.
impl FromRedisValue for Category {
    fn from_redis_value(v: &Value) -> RedisResult<Category> {
        Category::crate_from_redis_value(v, |c, k, v| {
            match k {
                b"id" => c.id = from_redis_value(v)?,
                b"name" => c.name = from_redis_value(v)?,
                b"thumbnail" => c.thumbnail = from_redis_value(v)?,
                b"topic_count" => c.topic_count = from_redis_value(v).ok(),
                b"post_count" => c.post_count = from_redis_value(v).ok(),
                b"topic_count_new" => c.topic_count_new = from_redis_value(v).ok(),
                b"post_count_new" => c.post_count_new = from_redis_value(v).ok(),
                _ => return Err((ErrorKind::ResponseError, "Response type not compatible").into()),
            };
            Ok(())
        })
    }
}

impl FromRedisValue for UserPSNProfile {
    fn from_redis_value(v: &Value) -> RedisResult<UserPSNProfile> {
        UserPSNProfile::crate_from_redis_value(v, |p, k, v| {
            match k {
                b"profile" => {
                    let v = from_redis_value::<Vec<u8>>(v)?;
                    let pp = serde_json::from_slice::<crate::model::psn::UserPSNProfile>(&v)
                        .map_err(|_| (ErrorKind::ResponseError, "Response type not compatible"))?;

                    p.id = pp.id;
                    p.online_id = pp.online_id;
                    p.np_id = pp.np_id;
                    p.region = pp.region;
                    p.avatar_url = pp.avatar_url;
                    p.about_me = pp.about_me;
                    p.languages_used = pp.languages_used;
                    p.plus = pp.plus;
                    p.level = pp.level;
                    p.progress = pp.progress;
                    p.platinum = pp.platinum;
                    p.gold = pp.gold;
                    p.silver = pp.silver;
                    p.bronze = pp.bronze;
                }
                _ => return Err((ErrorKind::ResponseError, "Response type not compatible").into()),
            };
            Ok(())
        })
    }
}

// work around to impl FromRedisValue for hashbrown::HashMap
pub struct HashMapBrown<K, V>(pub hashbrown::HashMap<K, V>);

impl<K, V> FromRedisValue for HashMapBrown<K, V>
where
    K: std::hash::Hash + FromRedisValue + Eq,
    V: FromRedisValue,
{
    fn from_redis_value(v: &Value) -> RedisResult<HashMapBrown<K, V>> {
        match *v {
            Value::Bulk(ref items) => {
                let mut rv = hashbrown::HashMap::default();
                let mut iter = items.iter();
                while let Some(k) = iter.next() {
                    if let Some(v) = iter.next() {
                        rv.insert(from_redis_value(k)?, from_redis_value(v)?);
                    }
                }
                Ok(HashMapBrown(rv))
            }
            _ => Err((
                ErrorKind::ResponseError,
                "Response type not hashbrown::HashMap compatible",
            )
                .into()),
        }
    }
}

impl FromRef<Topic> for Vec<(&str, Vec<u8>)> {
    fn from_ref(t: &Topic) -> Self {
        vec![("topic", serde_json::to_vec(t).unwrap_or_else(|_| vec![]))]
    }
}

impl FromRef<User> for Vec<(&str, Vec<u8>)> {
    fn from_ref(u: &User) -> Self {
        vec![("user", serde_json::to_vec(u).unwrap_or_else(|_| vec![]))]
    }
}

impl FromRef<Post> for Vec<(&str, Vec<u8>)> {
    fn from_ref(p: &Post) -> Self {
        vec![("post", serde_json::to_vec(p).unwrap_or_else(|_| vec![]))]
    }
}

impl FromRef<Category> for Vec<(&str, Vec<u8>)> {
    fn from_ref(c: &Category) -> Self {
        vec![
            ("id", c.id.to_string().into_bytes()),
            ("name", c.name.as_str().as_bytes().iter().copied().collect()),
            (
                "thumbnail",
                c.thumbnail.as_str().as_bytes().iter().copied().collect(),
            ),
        ]
    }
}

impl FromRef<UserPSNProfile> for Vec<(&str, Vec<u8>)> {
    fn from_ref(p: &UserPSNProfile) -> Self {
        vec![("profile", serde_json::to_vec(p).unwrap_or_else(|_| vec![]))]
    }
}

pub trait FromRef<T>: Sized {
    fn from_ref(t: &T) -> Self;
}

pub trait RefTo<T>: Sized {
    fn ref_to(&self) -> T;
}

impl<T, U> RefTo<U> for T
where
    U: FromRef<T>,
{
    fn ref_to(&self) -> U {
        U::from_ref(self)
    }
}

//  change to std::str::from_utf8 can get rid of unsafe functions.
fn parse_naive_date_time(t: &[u8]) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(
        unsafe { std::str::from_utf8_unchecked(t) },
        "%Y-%m-%d %H:%M:%S%.f",
    )
    .ok()
}

fn parse_count(c: &[u8]) -> Option<u32> {
    unsafe { std::str::from_utf8_unchecked(c) }
        .parse::<u32>()
        .ok()
}
