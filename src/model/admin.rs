use crate::model::{
	user::UserUpdateRequest,
	category::CategoryUpdateRequest,
	topic::TopicUpdateRequest,
	post::PostRequest,
};

pub enum AdminQuery<'a> {
	UpdateUserCheck(&'a u32, &'a UserUpdateRequest<'a>),
	UpdateCategoryCheck(&'a u32, &'a CategoryUpdateRequest<'a>),
	UpdateTopicCheck(&'a u32, &'a TopicUpdateRequest<'a>),
	UpdatePostCheck(&'a u32, &'a PostRequest<'a>),
	DeleteCategoryCheck(&'a u32, &'a u32),
}
