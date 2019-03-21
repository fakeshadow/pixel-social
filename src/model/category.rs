use actix::Message;
use crate::schema::categories;

use crate::model::errors::ServiceError;
use crate::model::topic::Topic;

#[derive(Identifiable, Queryable,Serialize)]
#[table_name = "categories"]
pub struct Category {
    pub id: i32,
    pub name: String,
    pub theme: String,
}

#[derive(Deserialize)]
pub struct CategoryRequest {
    pub categories: Vec<i32>,
    pub page: u32,
}

#[derive(Insertable)]
#[table_name = "categories"]
pub struct NewCategory<'a> {
    pub name: &'a str,
    pub theme: &'a str,
}

impl<'a> Category {
    fn new(name: &'a str, theme: &'a str) -> NewCategory<'a> {
        NewCategory {
            name,
            theme,
        }
    }
}

pub enum CategoryQuery {
    GetAllCategories,
    GetPopular(u32),
    GetCategory(CategoryRequest),
    AddCategory(CategoryRequest),
}

pub enum CategoryQueryResult {
    GotCategories(Vec<Category>),
    GotTopics(Vec<Topic>),
    AddedCategory,
}

impl Message for CategoryQuery {
    type Result = Result<CategoryQueryResult, ServiceError>;
}

impl CategoryQueryResult {
    pub fn to_topic_data(self) -> Option<Vec<Topic>> {
        match self {
            CategoryQueryResult::GotTopics(topics_data) => Some(topics_data),
            _ => None
        }
    }
    pub fn to_categories_data(self) -> Option<Vec<Category>> {
        match self {
            CategoryQueryResult::GotCategories(categories_data) => Some(categories_data),
            _ => None
        }
    }
}