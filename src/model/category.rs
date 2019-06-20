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

#[derive(Deserialize)]
pub struct CategoryRequest {
    pub id: Option<u32>,
    pub name: Option<String>,
    pub thumbnail: Option<String>,
}

impl GetSelfId for Category {
    fn get_self_id(&self) -> &u32 { &self.id }
}

impl CategoryRequest {
    pub fn make_category(mut self, id: u32) -> Result<Self, ServiceError> {
        if self.name.is_none() || self.thumbnail.is_none() {
            Err(ServiceError::BadRequest)
        } else {
            self.id = Some(id);
            Ok(self)
        }
    }
    pub fn make_update(self) -> Result<Self, ServiceError> {
        if self.id.is_none() {
            Err(ServiceError::BadRequest)
        } else {
            Ok(self)
        }
    }
}