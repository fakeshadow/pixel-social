#[derive(Deserialize)]
pub struct AdminCategoryJson {
    modify_type: u32,
    category_id: u32,
    category_data: String,
}

pub enum AdminQuery {
    ModifyCategory,
    UpdateUser,
    UpdateTopic,
    UpdatePost,
}

pub enum AdminQueryResult {
    Modified,
    Updated,
}
