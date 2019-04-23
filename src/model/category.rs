use crate::model::{
    errors::ServiceError,
    topic::TopicWithUser,
    user::SlimUser
};
use crate::schema::categories;

#[derive(Queryable, Serialize)]
pub struct Category {
    pub id: u32,
    pub name: String,
    pub theme: String
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
    pub category_id: Option<u32>,
    pub category_name: Option<String>,
    pub category_theme: Option<String>,
}

pub struct CategoryUpdateRequest<'a> {
    pub category_id: Option<&'a u32>,
    pub category_name: Option<&'a str>,
    pub category_theme: Option<&'a str>,
}

impl CategoryUpdateJson {
    pub fn to_request(&self) -> CategoryUpdateRequest {
        CategoryUpdateRequest {
            category_id: self.category_id.as_ref(),
            category_name: self.category_name.as_ref().map(String::as_str),
            category_theme: self.category_theme.as_ref().map(String::as_str),
        }
    }
}

#[derive(AsChangeset)]
#[table_name="categories"]
pub struct CategoryUpdateRequestInsert<'a> {
    pub name: Option<&'a str>,
    pub theme: Option<&'a str>,
}

impl<'a> CategoryUpdateRequest<'a> {
    pub fn insert(&self) -> CategoryUpdateRequestInsert {
        CategoryUpdateRequestInsert {
            name: self.category_name,
            theme: self.category_theme
        }
    }
}

pub enum CategoryQuery<'a> {
    GetAllCategories,
    GetPopular(i64),
    GetCategory(CategoryRequest<'a>),
    AddCategory(CategoryUpdateRequest<'a>),
    UpdateCategory(CategoryUpdateRequest<'a>),
    DeleteCategory(&'a u32)
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
