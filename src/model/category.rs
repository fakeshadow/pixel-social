use crate::model::{
    common::GetSelfId,
    errors::ServiceError,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Category {
    pub id: u32,
    pub name: String,
    pub thumbnail: String,
    // fields below stored only in redis. return None when querying database.
    pub topic_count: Option<u32>,
    pub post_count: Option<u32>,
    // new is last 24 hrs
    pub topic_count_new: Option<u32>,
    pub post_count_new: Option<u32>,
}

#[derive(Deserialize)]
pub struct CategoryRequest {
    pub id: Option<u32>,
    pub name: Option<String>,
    pub thumbnail: Option<String>,
}

impl GetSelfId for Category {
    fn self_id(&self) -> &u32 { &self.id }
}

impl CategoryRequest {
    pub fn check_new(&self) -> Result<(), ServiceError> {
        if self.name.is_none() || self.thumbnail.is_none() {
            Err(ServiceError::BadRequest)
        } else {
            Ok(())
        }
    }
    pub fn check_update(&self) -> Result<(), ServiceError> {
        if self.id.is_none() {
            Err(ServiceError::BadRequest)
        } else {
            Ok(())
        }
    }
}