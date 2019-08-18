use crate::model::{common::SelfIdString, errors::ResError};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Category {
    pub id: u32,
    pub name: String,
    pub thumbnail: String,
    // fields below stored only in redis. return None when querying database.
    pub topic_count: Option<u32>,
    pub post_count: Option<u32>,
    // new is last 24 hrs stores only in redis.
    pub topic_count_new: Option<u32>,
    pub post_count_new: Option<u32>,
}

impl Default for Category {
    fn default() -> Category {
        Category {
            id: 0,
            name: "".to_string(),
            thumbnail: "".to_string(),
            topic_count: None,
            post_count: None,
            topic_count_new: None,
            post_count_new: None,
        }
    }
}

#[derive(Deserialize)]
pub struct CategoryRequest {
    pub id: Option<u32>,
    pub name: Option<String>,
    pub thumbnail: Option<String>,
}

impl SelfIdString for Category {
    fn self_id_string(&self) -> String {
        self.id.to_string()
    }
}

impl CategoryRequest {
    pub fn check_new(&self) -> Result<(), ResError> {
        if self.name.is_none() || self.thumbnail.is_none() {
            Err(ResError::BadRequest)
        } else {
            Ok(())
        }
    }
    pub fn check_update(&self) -> Result<(), ResError> {
        if self.id.is_none() {
            Err(ResError::BadRequest)
        } else {
            Ok(())
        }
    }
}

#[derive(Deserialize, Debug)]
pub enum QueryType {
    Latest,
    Popular,
    PopularAll,
    All,
}

#[derive(Deserialize, Debug)]
pub struct CategoryQuery {
    pub category_id: Option<u32>,
    pub page: Option<usize>,
    pub query_type: QueryType,
}
