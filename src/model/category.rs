use actix::Message;
use crate::schema::categories;

use crate::model::errors::ServiceError;
use crate::model::topic::Topic;

#[derive(Identifiable, Queryable, Serialize)]
#[table_name = "categories"]
pub struct Category {
    pub id: i32,
    pub name: String,
    pub theme: String,
}

#[derive(Deserialize)]
pub struct CategoryRequest {
    pub categories: Option<Vec<i32>>,
    /// 0 add category, 1 modify category, 2 delete category
    pub modify_type: Option<u32>,
    pub category_id: Option<i32>,
    pub category_data: Option<CategoryData>,
    pub page: Option<i64>,
}

#[derive(Insertable, Deserialize, Clone)]
#[table_name = "categories"]
pub struct CategoryData {
    pub name: String,
    pub theme: String,
}

pub enum CategoryQuery {
    GetAllCategories,
    GetPopular(i64),
    GetCategory(CategoryRequest),
    ModifyCategory(CategoryRequest),
}

pub enum CategoryQueryResult {
    GotCategories(Vec<Category>),
    GotTopics(Vec<Topic>),
    ModifiedCategory,
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