use actix_web::HttpResponse;
use chrono::NaiveDateTime;

use crate::model::{
    user::SlimUser,
    errors::ServiceError,
    post::PostWithUser,
    common::{GetSelfId, GetSelfTimeStamp, MatchUser, SelfHaveField, ResponseMessage},
};
use crate::schema::topics;
use crate::model::common::CheckUserId;

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TopicWithUser<T> {
    #[serde(flatten)]
    pub topic: Topic,
    pub user: Option<T>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopicWithPost {
    pub topic: Option<TopicWithUser<SlimUser>>,
    pub posts: Option<Vec<PostWithUser>>,
}

impl MatchUser for Topic {
    fn get_user_id(&self) -> &u32 {
        &self.user_id
    }
}

impl<T> GetSelfId for TopicWithUser<T> {
    fn get_self_id(&self) -> &u32 {
        &self.topic.id
    }
    fn get_self_id_copy(&self) -> u32 { self.topic.id }
}

impl TopicWithPost {
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

impl SelfHaveField for TopicWithPost {
    fn have_topic(&self) -> bool {
        match &self.topic {
            Some(_topic) => true,
            None => false
        }
    }
    fn have_post(&self) -> bool {
        match &self.posts {
            Some(posts) => if !posts.is_empty() { true } else { false },
            None => false
        }
    }
}

impl<T> TopicWithUser<T>
    where T: GetSelfId {
    pub fn check_user_id(&self) -> Option<u32> {
        match &self.user {
            Some(user) => Some(user.get_self_id_copy()),
            None => None
        }
    }
}

/// extract self user and self topic from topic with user
impl CheckUserId<SlimUser, Topic> for TopicWithUser<SlimUser> {
    fn get_self_user(&self) -> Option<&SlimUser> {
        self.user.as_ref()
    }
    fn get_self_post_topic(&self) -> &Topic {
        &self.topic
    }
}

//impl<T> GetSelfTimeStamp for TopicWithUser<T> {
//    fn get_last_reply_time(&self) -> &NaiveDateTime { &self.topic.last_reply_time }
//}

impl Topic {
    pub fn attach_user<T>(self, users: &Vec<T>) -> TopicWithUser<T>
        where
            T: Clone + GetSelfId,
    {
        TopicWithUser {
            user: self.make_user_field(users),
            topic: self,
        }
    }
}

pub enum TopicQuery<'a> {
    GetTopic(&'a u32, &'a i64),
    AddTopic(&'a TopicRequest<'a>),
    UpdateTopic(&'a TopicRequest<'a>),
}

pub enum TopicQueryResult {
    ModifiedTopic,
    GotTopic(TopicWithPost),
}

impl TopicQueryResult {
    pub fn to_response(&self) -> HttpResponse {
        match self {
            TopicQueryResult::ModifiedTopic => HttpResponse::Ok().json(ResponseMessage::new("Add Topic Success")),
            TopicQueryResult::GotTopic(topic_with_post) => HttpResponse::Ok().json(&topic_with_post)
        }
    }
}