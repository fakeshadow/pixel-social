use crate::model::{errors::ServiceError, topic::TopicWithUser, user::SlimUser};
use crate::schema::categories;

#[derive(Queryable, Serialize)]
pub struct Category {
    pub id: u32,
    pub name: Option<String>,
    pub theme: Option<String>,
}

#[derive(Insertable)]
#[table_name = "categories"]
pub struct NewCategory<'a> {
    pub id: u32,
    pub name: Option<&'a str>,
    pub theme: Option<&'a str>
}

impl<'a> Category {
    pub fn new(id: u32, name: &'a str, theme: &'a str) -> NewCategory<'a> {
        NewCategory{
            id,
            name: Some(name),
            theme: Some(theme)
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
    pub modify_type: u32,
    pub category_id: Option<u32>,
    pub category_name: Option<String>,
    pub category_theme: Option<String>,
}


pub struct CategoryUpdateRequest<'a> {
    pub modify_type: &'a u32,
    pub category_id: Option<&'a u32>,
    pub category_name: Option<&'a String>,
    pub category_theme: Option<&'a String>,
}

pub enum CategoryQuery<'a> {
    GetAllCategories,
    GetPopular(i64),
    GetCategory(CategoryRequest<'a>),
    UpdateCategory(CategoryUpdateRequest<'a>),
}

pub enum CategoryQueryResult {
    GotCategories(Vec<Category>),
    GotTopics(Vec<TopicWithUser<SlimUser>>),
    UpdatedCategory,
}

// test use
pub enum CategoryQueryTest {
    GetCategory(CategoryRequestTest),
}

pub struct CategoryRequestTest {
    pub categories: Vec<u32>,
    pub page: i64,
}
