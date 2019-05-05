use crate::model::{
    errors::ServiceError,
    topic::{Topic,TopicWithUser},
    common::{GetSelfId}
};
use crate::schema::categories;

#[derive(Queryable, Serialize, Deserialize, Debug)]
pub struct Category {
    pub id: u32,
    pub name: String,
    pub topic_count: i32,
    pub post_count: i32,
    pub subscriber_count: i32,
    pub thumbnail: String,
}

impl GetSelfId for Category {
    fn get_self_id(&self) -> &u32 { &self.id }
}

#[derive(Insertable, Debug)]
#[table_name = "categories"]
pub struct NewCategory<'a> {
    pub id: &'a u32,
    pub name: &'a str,
    pub thumbnail: &'a str,
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

impl CategoryUpdateJson {
    pub fn to_request(&self) -> CategoryUpdateRequest {
        CategoryUpdateRequest {
            category_id: self.category_id.as_ref(),
            category_name: self.category_name.as_ref().map(String::as_str),
            category_thumbnail: self.category_thumbnail.as_ref().map(String::as_str),
        }
    }
}

pub struct CategoryUpdateRequest<'a> {
    pub category_id: Option<&'a u32>,
    pub category_name: Option<&'a str>,
    pub category_thumbnail: Option<&'a str>,
}

impl<'a> CategoryUpdateRequest<'a> {
    pub fn make_category(&'a self, id: &'a u32) -> Result<NewCategory<'a>, ServiceError> {
        Ok(NewCategory {
            id,
            name: self.category_name.ok_or(ServiceError::BadRequestGeneral)?,
            thumbnail: self.category_thumbnail.ok_or(ServiceError::BadRequestGeneral)?,
        })
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
    GetPopular(&'a i64),
    GetCategory(&'a CategoryRequest<'a>),
    AddCategory(&'a CategoryUpdateRequest<'a>),
    UpdateCategory(&'a CategoryUpdateRequest<'a>),
    DeleteCategory(&'a u32),
}
