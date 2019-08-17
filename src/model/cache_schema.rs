use redis::{from_redis_value, ErrorKind, FromRedisValue, RedisResult, Value};

use crate::model::{category::Category, post::Post, psn::UserPSNProfile, topic::Topic, user::User};

/// any from redis value error will lead to a database query.
/// Cache failure could potential be fixed after that.
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

impl FromRedisValue for UserPSNProfile {
    fn from_redis_value(v: &Value) -> RedisResult<UserPSNProfile> {
        UserPSNProfile::parse_from_redis_value(v)
    }
}

trait ParseFromRedisValue {
    type Result;
    fn parse_from_redis_value(v: &Value) -> Result<Self::Result, redis::RedisError>
    where
        Self::Result: Default + std::fmt::Debug,
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
                    if let Err(e) = Self::parse_pattern(&mut t, key.as_str(), v) {
                        return Err(e);
                    }
                }
                Ok(t)
            }
            _ => Err((ErrorKind::ResponseError, "Response type not compatible"))?,
        }
    }

    fn parse_pattern(t: &mut Self::Result, key: &str, v: &Value) -> Result<(), redis::RedisError>;
}

impl ParseFromRedisValue for Topic {
    type Result = Topic;
    fn parse_pattern(t: &mut Topic, k: &str, v: &Value) -> Result<(), redis::RedisError> {
        match k {
            "topic" => {
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
            _ => return Err((ErrorKind::ResponseError, "Response type not compatible"))?,
        };
        Ok(())
    }
}

impl ParseFromRedisValue for Post {
    type Result = Post;
    fn parse_pattern(p: &mut Post, k: &str, v: &Value) -> Result<(), redis::RedisError> {
        match k {
            "post" => {
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
            _ => return Err((ErrorKind::ResponseError, "Response type not compatible"))?,
        };
        Ok(())
    }
}

impl ParseFromRedisValue for User {
    type Result = User;
    fn parse_pattern(u: &mut User, k: &str, v: &Value) -> Result<(), redis::RedisError> {
        match k {
            "user" => {
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

impl ParseFromRedisValue for UserPSNProfile {
    type Result = UserPSNProfile;
    fn parse_pattern(p: &mut UserPSNProfile, k: &str, v: &Value) -> Result<(), redis::RedisError> {
        match k {
            "profile" => {
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
            _ => return Err((ErrorKind::ResponseError, "Response type not compatible"))?,
        };
        Ok(())
    }
}

impl Into<Vec<(&str, Vec<u8>)>> for Topic {
    fn into(self) -> Vec<(&'static str, Vec<u8>)> {
        vec![(
            "topic",
            serde_json::to_vec(&self).unwrap_or_else(|_| [].to_vec()),
        )]
    }
}

impl Into<Vec<(&str, Vec<u8>)>> for User {
    fn into(self) -> Vec<(&'static str, Vec<u8>)> {
        vec![(
            "user",
            serde_json::to_vec(&self).unwrap_or_else(|_| [].to_vec()),
        )]
    }
}

impl Into<Vec<(&str, Vec<u8>)>> for Post {
    fn into(self) -> Vec<(&'static str, Vec<u8>)> {
        vec![(
            "post",
            serde_json::to_vec(&self).unwrap_or_else(|_| [].to_vec()),
        )]
    }
}

impl Into<Vec<(&str, Vec<u8>)>> for Category {
    fn into(self) -> Vec<(&'static str, Vec<u8>)> {
        vec![
            ("id", self.id.to_string().into_bytes()),
            ("name", self.name.into_bytes()),
            ("thumbnail", self.thumbnail.into_bytes()),
        ]
    }
}

impl Into<Vec<(&str, Vec<u8>)>> for UserPSNProfile {
    fn into(self) -> Vec<(&'static str, Vec<u8>)> {
        vec![(
            "profile",
            serde_json::to_vec(&self).unwrap_or_else(|_| [].to_vec()),
        )]
    }
}
