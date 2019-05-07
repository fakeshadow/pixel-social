use crate::model::{
    category::CategoryUpdateRequest,
    post::PostRequest,
    topic::TopicRequest,
    user::UserUpdateRequest,
};

pub enum AdminPrivilegeCheck<'a> {
	UpdateUserCheck(&'a u32, &'a UserUpdateRequest),
	UpdateCategoryCheck(&'a u32, &'a CategoryUpdateRequest),
	UpdateTopicCheck(&'a u32, &'a TopicRequest),
	UpdatePostCheck(&'a u32, &'a PostRequest),
	DeleteCategoryCheck(&'a u32),
}

pub trait IdToQuery {
	fn to_privilege_check<'a>(&self, jwt_id: &'a u32) -> AdminPrivilegeCheck<'a>;
}

impl IdToQuery for u32 {
	fn to_privilege_check<'a>(&self, jwt_id: &'a u32) -> AdminPrivilegeCheck<'a> {
		AdminPrivilegeCheck::DeleteCategoryCheck(jwt_id)
	}
}