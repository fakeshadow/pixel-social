use crate::model::{
	user::UserUpdateRequest,
	category::CategoryUpdateRequest,
	topic::TopicRequest,
	post::PostRequest,
};

pub enum AdminPrivilegeCheck<'a> {
	UpdateUserCheck(&'a u32, &'a UserUpdateRequest<'a>),
	UpdateCategoryCheck(&'a u32, &'a CategoryUpdateRequest<'a>),
	UpdateTopicCheck(&'a u32, &'a TopicRequest),
	UpdatePostCheck(&'a u32, &'a PostRequest),
	DeleteCategoryCheck(&'a u32),
}
