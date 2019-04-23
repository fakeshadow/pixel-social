use actix_web::HttpResponse;

use crate::model::{
    errors::ServiceError,
    topic::TopicWithUser,
    common::ResponseMessage,
    user::SlimUser,
};
use crate::schema::categories;


#[derive(Queryable, Serialize)]
pub struct Category {
    pub id: u32,
    pub name: String,
    pub topic_count: u32,
    pub post_count:u32,
    pub subscriber_count: u32,
    pub thumbnail: String,
}

#[derive(Insertable)]
#[table_name = "categories"]
pub struct NewCategory<'a> {
    pub id: u32,
    pub name: Option<&'a str>,
    pub thumbnail: Option<&'a str>,
}

impl<'a> Category {
    pub fn new(id: u32, name: &'a str, thumbnail: &'a str) -> NewCategory<'a> {
        NewCategory {
            id,
            name: Some(name),
            thumbnail: Some(thumbnail),
        }
    }
}

#[derive(Deserialize)]
pub struct CategoryJson {
    pub categories: Vec<u32>,
    pub page: i64,
}

pub struct CategoryRequest<'a> {
    pub categories: &'a Vec<u32>,
    pub page: &'a i64,
}

#[derive(Deserialize)]
pub struct CategoryUpdateJson {
    pub category_id: Option<u32>,
    pub category_name: Option<String>,
    pub category_thumbnail: Option<String>,
}

pub struct CategoryUpdateRequest<'a> {
    pub category_id: Option<&'a u32>,
    pub category_name: Option<&'a str>,
    pub category_thumbnail: Option<&'a str>,
}

impl CategoryUpdateJson {
    pub fn to_request(&self) -> CategoryUpdateRequest {
        CategoryUpdateRequest {
            category_id: self.category_id.as_ref(),
            category_name: self.category_name.as_ref().map(String::as_str),
            category_thumbnail: self.category_thumbnail.as_ref().map(String::as_str),
        }
    }
}

#[derive(AsChangeset)]
#[table_name = "categories"]
pub struct CategoryUpdateRequestInsert<'a> {
    pub name: Option<&'a str>,
    pub thumbnail: Option<&'a str>,
}

impl<'a> CategoryUpdateRequest<'a> {
    pub fn insert(&self) -> CategoryUpdateRequestInsert {
        CategoryUpdateRequestInsert {
            name: self.category_name,
            thumbnail: self.category_thumbnail,
        }
    }
}

pub enum CategoryQuery<'a> {
    GetAllCategories,
    GetPopular(i64),
    GetCategory(&'a CategoryRequest<'a>),
    AddCategory(&'a CategoryUpdateRequest<'a>),
    UpdateCategory(&'a CategoryUpdateRequest<'a>),
    DeleteCategory(&'a u32),
}

pub enum CategoryQueryResult {
    GotCategories(Vec<Category>),
    GotTopics(Vec<TopicWithUser<SlimUser>>),
    UpdatedCategory,
}

impl CategoryQueryResult {
    pub fn to_response(&self) -> HttpResponse {
        match self {
            CategoryQueryResult::GotCategories(categories) => HttpResponse::Ok().json(&categories),
            CategoryQueryResult::GotTopics(topics) => HttpResponse::Ok().json(&topics),
            CategoryQueryResult::UpdatedCategory => HttpResponse::Ok().json(ResponseMessage::new("Modify Success"))
        }
    }
}
