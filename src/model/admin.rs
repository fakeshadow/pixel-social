use crate::model::{
	user::UserUpdateRequest,
	category::CategoryUpdateRequest,
	topic::TopicRequest,
	post::PostRequest,
};

pub enum AdminQuery<'a> {
	UpdateUserCheck(&'a u32, &'a UserUpdateRequest<'a>),
	UpdateCategoryCheck(&'a u32, &'a CategoryUpdateRequest<'a>),
	UpdateTopicCheck(&'a u32, &'a TopicRequest<'a>),
	UpdatePostCheck(&'a u32, &'a PostRequest<'a>),
	DeleteCategoryCheck(&'a u32),
}
