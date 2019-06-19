use crate::model::{
    common::GetSelfId,
    errors::ServiceError,
};

#[derive(Serialize, Deserialize, Clone)]
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

pub struct NewCategory<'a> {
    pub id: &'a u32,
    pub name: &'a str,
    pub thumbnail: &'a str,
}

pub struct UpdateCategory<'a> {
    pub id: u32,
    pub name: Option<&'a str>,
    pub thumbnail: Option<&'a str>,
}

#[derive(Deserialize)]
pub struct CategoryUpdateRequest {
    pub category_id: Option<u32>,
    pub category_name: Option<String>,
    pub category_thumbnail: Option<String>,
}

impl CategoryUpdateRequest {
    pub fn make_category<'a>(&'a self, id: &'a u32) -> Result<NewCategory<'a>, ServiceError> {
        Ok(NewCategory {
            id,
            name: self.category_name.as_ref().ok_or(ServiceError::BadRequest)?,
            thumbnail: self.category_thumbnail.as_ref().ok_or(ServiceError::BadRequest)?,
        })
    }
    pub fn make_update(&self) -> Result<UpdateCategory, ServiceError> {
        Ok(UpdateCategory {
            id: self.category_id.ok_or(ServiceError::BadRequest)?,
            name: self.category_name.as_ref().map(String::as_str),
            thumbnail: self.category_thumbnail.as_ref().map(String::as_str),
        })
    }
}