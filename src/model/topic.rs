use chrono::NaiveDateTime;

use crate::model::{
    admin::AdminPrivilegeCheck,
    common::{AttachUser, GetSelfId, GetUserId},
    errors::ServiceError,
    post::{Post, PostWithUser},
    user::{ToUserRef, User, UserRef},
};
use crate::schema::topics;

#[derive(Queryable, Serialize, Debug, Clone)]
pub struct Topic {
    pub id: u32,
    pub user_id: u32,
    pub category_id: u32,
    pub title: String,
    pub body: String,
    pub thumbnail: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub last_reply_time: NaiveDateTime,
    pub reply_count: i32,
    pub is_locked: bool,
}

#[derive(Insertable)]
#[table_name = "topics"]
pub struct NewTopic<'a> {
    pub id: &'a u32,
    pub user_id: &'a u32,
    pub category_id: &'a u32,
    pub thumbnail: &'a str,
    pub title: &'a str,
    pub body: &'a str,
}

#[derive(AsChangeset)]
#[table_name = "topics"]
pub struct UpdateTopic<'a> {
    pub id: &'a u32,
    pub user_id: Option<&'a u32>,
    pub category_id: Option<&'a u32>,
    pub title: Option<&'a str>,
    pub body: Option<&'a str>,
    pub thumbnail: Option<&'a str>,
    pub is_locked: Option<&'a bool>,
}

#[derive(Deserialize)]
pub struct TopicRequest {
    pub id: Option<u32>,
    pub user_id: Option<u32>,
    pub category_id: Option<u32>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub thumbnail: Option<String>,
    pub is_locked: Option<bool>,
}

impl TopicRequest {
    pub fn attach_user_id(&mut self, id: Option<u32>) -> &Self {
        self.user_id = id;
        self
    }
    pub fn attach_user_id_into(mut self, id: Option<u32>) -> Self {
        self.user_id = id;
        self
    }

    pub fn to_privilege_check<'a>(&'a self, level: &'a u32) -> AdminPrivilegeCheck<'a> {
        AdminPrivilegeCheck::UpdateTopicCheck(level, self)
    }

    pub fn into_add_query(self) -> TopicQuery { TopicQuery::AddTopic(self) }
    pub fn into_update_query(self) -> TopicQuery { TopicQuery::UpdateTopic(self) }


    pub fn extract_self_id(&self) -> Result<&u32, ServiceError> {
        Ok(self.id.as_ref().ok_or(ServiceError::BadRequestGeneral)?)
    }
    pub fn extract_category_id(&self) -> Result<&u32, ServiceError> {
        Ok(self.category_id.as_ref().ok_or(ServiceError::BadRequestGeneral)?)
    }

    pub fn make_topic<'a>(&'a self, id: &'a u32) -> Result<NewTopic<'a>, ServiceError> {
        Ok(NewTopic {
            id,
            user_id: self.user_id.as_ref().ok_or(ServiceError::BadRequestGeneral)?,
            category_id: self.extract_category_id()?,
            thumbnail: self.thumbnail.as_ref().ok_or(ServiceError::BadRequestGeneral)?,
            title: self.title.as_ref().ok_or(ServiceError::BadRequestGeneral)?,
            body: self.body.as_ref().ok_or(ServiceError::BadRequestGeneral)?,
        })
    }
    pub fn make_update(&self) -> Result<UpdateTopic, ServiceError> {
        match self.user_id {
            Some(_) => Ok(UpdateTopic {
                id: self.extract_self_id()?,
                user_id: self.user_id.as_ref(),
                category_id: None,
                title: self.title.as_ref().map(String::as_str),
                body: self.body.as_ref().map(String::as_str),
                thumbnail: self.thumbnail.as_ref().map(String::as_str),
                is_locked: None,
            }),
            None => Ok(UpdateTopic {
                id: self.extract_self_id()?,
                user_id: None,
                category_id: self.category_id.as_ref(),
                title: self.title.as_ref().map(String::as_str),
                body: self.body.as_ref().map(String::as_str),
                thumbnail: self.thumbnail.as_ref().map(String::as_str),
                is_locked: self.is_locked.as_ref(),
            })
        }
    }
}

impl GetSelfId for Topic {
    fn get_self_id(&self) -> &u32 { &self.id }
}

impl<'u, T> AttachUser<'u, T> for Topic
    where T: GetSelfId + ToUserRef {
    type Output = TopicWithUser<'u>;
    fn self_user_id(&self) -> &u32 { &self.user_id }
    fn attach_user(&'u self, users: &'u Vec<T>) -> Self::Output {
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

impl<'a> TopicWithPost<'a> {
    pub fn new(t: Option<&'a Topic>, p: &'a Vec<Post>, u: &'a Vec<User>) -> Self {
        TopicWithPost {
            topic: t.map(|t| t.attach_user(&u)),
            posts: p.iter().map(|p| p.attach_user(&u)).collect(),
        }
    }
}

impl GetUserId for Topic {
    fn get_user_id(&self) -> u32 { self.user_id }
}

pub enum TopicQuery {
    GetTopic(u32, i64),
    GetTopics(Vec<u32>, i64),
    AddTopic(TopicRequest),
    UpdateTopic(TopicRequest),
}

pub trait PathToQuery {
    fn to_query(&self) -> TopicQuery;
}

impl PathToQuery for (u32, i64) {
    fn to_query(&self) -> TopicQuery {
        TopicQuery::GetTopic(self.0, self.1)
    }
}