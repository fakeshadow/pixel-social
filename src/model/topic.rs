use actix_web::HttpResponse;
use chrono::NaiveDateTime;

use crate::model::{
    user::PublicUser,
    errors::ServiceError,
    common::{GetSelfId, ResponseMessage},
};
use crate::schema::topics;
use crate::model::user::{User, PublicUserRef, ToPublicUserRef};
use crate::model::common::{AttachPublicUserRef, GetUserId};
use crate::model::post::PostWithUser;

#[derive(Debug, Queryable, Serialize, Deserialize, Clone)]
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

#[derive(Serialize)]
pub struct TopicRef<'a> {
    pub id: &'a u32,
    pub user_id: &'a u32,
    pub category_id: &'a u32,
    pub title: &'a str,
    pub body: &'a str,
    pub thumbnail: &'a str,
    pub created_at: &'a NaiveDateTime,
    pub updated_at: &'a NaiveDateTime,
    pub last_reply_time: &'a NaiveDateTime,
    pub reply_count: &'a i32,
    pub is_locked: &'a bool,
}

impl Topic {
    pub fn to_ref(&self) -> TopicRef {
        TopicRef {
            id: &self.id,
            user_id: &self.user_id,
            category_id: &self.category_id,
            title: &self.title,
            body: &self.body,
            thumbnail: &self.thumbnail,
            created_at: &self.created_at,
            updated_at: &self.updated_at,
            last_reply_time: &self.last_reply_time,
            reply_count: &self.reply_count,
            is_locked: &self.is_locked,
        }
    }
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
pub struct TopicJson {
    pub id: Option<u32>,
    pub user_id: Option<u32>,
    pub category_id: Option<u32>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub thumbnail: Option<String>,
    pub is_locked: Option<bool>,
}

impl<'a> TopicJson {
    pub fn to_request(&'a self, user_id: Option<&'a u32>) -> TopicRequest<'a> {
        TopicRequest {
            id: self.id.as_ref(),
            user_id,
            category_id: self.category_id.as_ref(),
            title: self.title.as_ref().map(String::as_str),
            body: self.body.as_ref().map(String::as_str),
            thumbnail: self.thumbnail.as_ref().map(String::as_str),
            is_locked: self.is_locked.as_ref(),
        }
    }
}

pub struct TopicRequest<'a> {
    pub id: Option<&'a u32>,
    pub user_id: Option<&'a u32>,
    pub category_id: Option<&'a u32>,
    pub title: Option<&'a str>,
    pub body: Option<&'a str>,
    pub thumbnail: Option<&'a str>,
    pub is_locked: Option<&'a bool>,
}

impl<'a> TopicRequest<'a> {
    pub fn extract_self_id(&self) -> Result<&'a u32, ServiceError> {
        Ok(self.id.ok_or(ServiceError::BadRequestGeneral)?)
    }

    pub fn extract_category_id(&self) -> Result<&'a u32, ServiceError> {
        Ok(self.category_id.ok_or(ServiceError::BadRequestGeneral)?)
    }

    pub fn make_topic(&'a self, id: &'a u32) -> Result<NewTopic<'a>, ServiceError> {
        Ok(NewTopic {
            id,
            user_id: self.user_id.ok_or(ServiceError::BadRequestGeneral)?,
            category_id: self.extract_category_id()?,
            thumbnail: self.thumbnail.ok_or(ServiceError::BadRequestGeneral)?,
            title: self.title.ok_or(ServiceError::BadRequestGeneral)?,
            body: self.body.ok_or(ServiceError::BadRequestGeneral)?,
        })
    }

    pub fn make_update(&'a self) -> Result<UpdateTopic, ServiceError> {
        match self.user_id {
            Some(id) => Ok(UpdateTopic {
                id: self.extract_self_id()?,
                user_id: self.user_id,
                category_id: None,
                title: self.title,
                body: self.body,
                thumbnail: self.thumbnail,
                is_locked: None,
            }),
            None => Ok(UpdateTopic {
                id: self.extract_self_id()?,
                user_id: None,
                category_id: self.category_id,
                title: self.title,
                body: self.body,
                thumbnail: self.thumbnail,
                is_locked: self.is_locked,
            })
        }
    }
}

impl<'a> GetSelfId for TopicRef<'a> {
    fn get_self_id(&self) -> &u32 { &self.id }
}

impl<'u, T> AttachPublicUserRef<'u, T> for TopicRef<'u>
    where T: GetSelfId + ToPublicUserRef {
    type Output = TopicWithUser<'u>;
    fn get_user_id(&self) -> &u32 {
        &self.user_id
    }
    fn attach_user(self, users: &'u Vec<T>) -> Self::Output {
        TopicWithUser {
            user: self.make_field(&users),
            topic: self,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct TopicWithUserTest {
    #[serde(flatten)]
    pub topic: Topic,
    pub user: Option<User>,
}

#[derive(Serialize)]
pub struct TopicWithUser<'a> {
    #[serde(flatten)]
    pub topic: TopicRef<'a>,
    pub user: Option<PublicUserRef<'a>>,
}

#[derive(Serialize)]
pub struct TopicWithPost<'a> {
    pub topic: Option<&'a TopicWithUser<'a>>,
    pub posts: Option<&'a Vec<PostWithUser<'a>>>,
}

impl<'a> TopicWithPost<'a> {
    pub fn new(topic: Option<&'a TopicWithUser<'a>>, posts: Option<&'a Vec<PostWithUser<'a>>>) -> Self {
        TopicWithPost { topic, posts }
    }
    pub fn get_topic_id(&self) -> Option<&u32> {
        match &self.posts {
            Some(posts) => Some(&posts[0].post.topic_id),
            None => None
        }
    }
    pub fn get_category_id(&self) -> Option<&u32> {
        match &self.topic {
            Some(topic) => Some(&topic.topic.category_id),
            None => None
        }
    }
}

impl GetUserId for Topic {
    fn get_user_id(&self) -> &u32 { &self.user_id }
}

//impl<T> GetSelfTimeStamp for TopicWithUser<T> {
//    fn get_last_reply_time(&self) -> &NaiveDateTime { &self.topic.last_reply_time }
//}

pub enum TopicQuery<'a> {
    GetTopic(&'a u32, &'a i64),
    AddTopic(&'a TopicRequest<'a>),
    UpdateTopic(&'a TopicRequest<'a>),
}

pub enum TopicQueryResult<'a> {
    ModifiedTopic,
    GotTopic(&'a TopicWithPost<'a>),
}

impl<'a> TopicQueryResult<'a> {
    pub fn to_response(&self) -> HttpResponse {
        match self {
            TopicQueryResult::ModifiedTopic => HttpResponse::Ok().json(ResponseMessage::new("Add Topic Success")),
            TopicQueryResult::GotTopic(topic_with_post) => HttpResponse::Ok().json(&topic_with_post)
        }
    }
}