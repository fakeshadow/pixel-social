use crate::model::{
	user::UserUpdateRequest,
	category::CategoryUpdateRequest,
	topic::TopicUpdateRequest,
	post::PostUpdateRequest,
};

pub enum AdminQuery<'a> {
	UpdateUserCheck(&'a u32, &'a UserUpdateRequest<'a>),
	UpdateCategoryCheck(&'a u32, &'a CategoryUpdateRequest<'a>),
	UpdateTopicCheck(&'a u32, &'a TopicUpdateRequest<'a>),
	UpdatePostCheck(&'a u32, &'a PostUpdateRequest<'a>),
	DeleteCategoryCheck(&'a u32, &'a u32),
}
