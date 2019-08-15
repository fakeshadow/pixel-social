use chrono::NaiveDateTime;

use crate::model::{
    common::{GetSelfId, GetUserId},
    errors::ResError,
    post::{Post, PostWithUser},
    user::{AttachUser, User, UserRef},
};


#[derive(Serialize, Deserialize, Debug)]
pub struct Topic {
    pub id: u32,
    pub user_id: u32,
    pub category_id: u32,
    pub title: String,
    pub body: String,
    pub thumbnail: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub is_locked: bool,
    pub is_visible: bool,
    // last_reply_time stores only in redis and will return none if query database
    pub last_reply_time: Option<NaiveDateTime>,
    // the same as last_reply_time only stores in redis.
    pub reply_count: Option<u32>,
}

impl Default for Topic {
    fn default() -> Topic {
        Topic {
            id: 0,
            user_id: 0,
            category_id: 0,
            title: "".to_string(),
            body: "".to_string(),
            thumbnail: "".to_string(),
            created_at: NaiveDateTime::from_timestamp(0, 0),
            updated_at: NaiveDateTime::from_timestamp(0, 0),
            is_locked: false,
            is_visible: true,
            last_reply_time: None,
            reply_count: None,
        }
    }
}

impl Topic {
    pub fn attach_users_with_post<'a>(
        t: Option<&'a Topic>,
        p: &'a Vec<Post>,
        u: &'a Vec<User>,
    ) -> TopicWithPost<'a> {
        TopicWithPost {
            topic: t.map(|t| t.attach_user(&u)),
            posts: p.iter().map(|p| p.attach_user(&u)).collect(),
        }
    }
    pub fn attach_users<'a>(t: &'a Vec<Topic>, u: &'a Vec<User>) -> Vec<TopicWithUser<'a>> {
        t.iter().map(|t| t.attach_user(&u)).collect()
    }
}

// handle incoming json request.
#[derive(Deserialize)]
pub struct TopicRequest {
    pub id: Option<u32>,
    pub user_id: Option<u32>,
    pub category_id: u32,
    pub title: Option<String>,
    pub body: Option<String>,
    pub thumbnail: Option<String>,
    pub is_locked: Option<bool>,
    pub is_visible: Option<bool>,
}

impl TopicRequest {
    pub fn attach_user_id(mut self, id: Option<u32>) -> Self {
        self.user_id = id;
        self
    }
    pub fn check_new(&self) -> Result<(), ResError> {
        if self.title.is_none() || self.body.is_none() || self.thumbnail.is_none() {
            Err(ResError::BadRequest)
        } else {
            Ok(())
        }
    }
    pub fn check_update(&mut self) -> Result<(), ResError> {
        if self.id.is_none() {
            return Err(ResError::BadRequest);
        }
        if let Some(_) = self.user_id {
            self.is_locked = None;
        }
        Ok(())
    }
}

impl GetSelfId for Topic {
    fn self_id(&self) -> &u32 {
        &self.id
    }
}

impl GetUserId for Topic {
    fn get_user_id(&self) -> u32 {
        self.user_id
    }
}

impl<'u> AttachUser<'u> for Topic {
    type Output = TopicWithUser<'u>;
    fn self_user_id(&self) -> &u32 {
        &self.user_id
    }
    fn attach_user(&'u self, users: &'u Vec<User>) -> Self::Output {
        TopicWithUser {
            user: self.make_field(&users),
            topic: self,
        }
    }
}

#[derive(Serialize)]
pub struct TopicWithUser<'a> {
    #[serde(flatten)]
    pub topic: &'a Topic,
    pub user: Option<UserRef<'a>>,
}

#[derive(Serialize)]
pub struct TopicWithPost<'a> {
    pub topic: Option<TopicWithUser<'a>>,
    pub posts: Vec<PostWithUser<'a>>,
}
