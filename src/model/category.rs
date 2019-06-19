use crate::model::{
    common::GetSelfId,
    errors::ServiceError,
    topic::{Topic, TopicWithUser},
};
use crate::model::admin::AdminPrivilegeCheck;
use crate::schema::categories;

#[derive(Queryable, Serialize, Deserialize, Clone)]
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

#[derive(Insertable)]
#[table_name = "categories"]
pub struct NewCategory<'a> {
    pub id: &'a u32,
    pub name: &'a str,
    pub thumbnail: &'a str,
}

#[derive(AsChangeset)]
#[table_name = "categories"]
pub struct UpdateCategory<'a> {
    pub name: Option<&'a str>,
    pub thumbnail: Option<&'a str>,
}

#[derive(Deserialize)]
pub struct CategoryRequest {
    pub categories: Vec<u32>,
    pub page: i64,
}


#[derive(Deserialize)]
pub struct CategoryUpdateRequest {
    pub category_id: Option<u32>,
    pub category_name: Option<String>,
    pub category_thumbnail: Option<String>,
}

impl CategoryUpdateRequest {
    pub fn to_privilege_check<'a>(&'a self, level: &'a u32) -> AdminPrivilegeCheck<'a> {
        AdminPrivilegeCheck::UpdateCategoryCheck(level, self)
    }
    pub fn into_add_query(self) -> CategoryQuery { CategoryQuery::AddCategory(self) }
    pub fn into_update_query(self) -> CategoryQuery { CategoryQuery::UpdateCategory(self) }

    pub fn make_category<'a>(&'a self, id: &'a u32) -> Result<NewCategory<'a>, ServiceError> {
        Ok(NewCategory {
            id,
            name: self.category_name.as_ref().ok_or(ServiceError::BadRequest)?,
            thumbnail: self.category_thumbnail.as_ref().ok_or(ServiceError::BadRequest)?,
        })
    }
    pub fn make_update(&self) -> UpdateCategory {
        UpdateCategory {
            name: self.category_name.as_ref().map(String::as_str),
            thumbnail: self.category_thumbnail.as_ref().map(String::as_str),
        }
    }
}

pub enum CategoryQuery {
    GetAllCategories,
    GetPopular(i64),
    AddCategory(CategoryUpdateRequest),
    UpdateCategory(CategoryUpdateRequest),
    DeleteCategory(u32),
}

pub trait IdToQuery {
    fn to_delete_query(&self) -> CategoryQuery;
}

impl IdToQuery for u32 {
    fn to_delete_query(&self) -> CategoryQuery {
        CategoryQuery::DeleteCategory(*self)
    }
}