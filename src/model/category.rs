use crate::schema::categories;
use crate::model::{
    errors::ServiceError,
    user::SlimmerUser,
    topic::{TopicWithUser}
};

#[derive(Identifiable, Queryable, Serialize)]
#[table_name = "categories"]
pub struct Category {
    pub id: u32,
    pub name: String,
    pub theme: String,
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


#[derive(Insertable, Deserialize, Clone)]
#[table_name = "categories"]
pub struct CategoryData {
    pub name: String,
    pub theme: String,
}


pub enum CategoryQuery<'a> {
    GetAllCategories,
    GetPopular(i64),
    GetCategory(CategoryRequest<'a>),
    ModifyCategory(CategoryRequest<'a>),
}

pub enum CategoryQueryResult {
    GotCategories(Vec<Category>),
    GotTopics(Vec<TopicWithUser<SlimmerUser>>),
    ModifiedCategory,
}
