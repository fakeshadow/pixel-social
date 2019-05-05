use actix_web::web;
use diesel::prelude::*;

use crate::model::{
    user::{UserUpdateRequest},
    category::CategoryUpdateRequest,
    topic::TopicRequest,
    post::PostRequest,
    admin::AdminPrivilegeCheck,
    errors::ServiceError,
    common::{PostgresPool, PoolConnectionPostgres},
};
use crate::handler::user::get_user_by_id;

type QueryResult = Result<(), ServiceError>;


impl<'a> AdminPrivilegeCheck<'a> {
    pub fn handle_check(self, db: &PostgresPool) -> QueryResult {
        let conn = &db.get().unwrap();
        match self {
            AdminPrivilegeCheck::UpdateUserCheck(lv, req) => update_user_check(&lv, &req, conn),
            AdminPrivilegeCheck::UpdateCategoryCheck(lv, req) => update_category_check(&lv, &req),
            AdminPrivilegeCheck::UpdateTopicCheck(lv, req) => update_topic_check(&lv, &req),
            AdminPrivilegeCheck::UpdatePostCheck(lv, req) => update_post_check(&lv, &req),
            AdminPrivilegeCheck::DeleteCategoryCheck(lv) => if lv < &9 { Err(ServiceError::Unauthorized) } else { Ok(()) }
        }
    }
}

fn update_user_check(lv: &u32, req: &UserUpdateRequest, conn: &PoolConnectionPostgres) -> QueryResult {
    check_admin_level(&req.is_admin, &lv, 9)?;
    let user = get_user_by_id(&req.id, conn)?.pop().ok_or(ServiceError::BadRequestGeneral)?;
    if lv <= &user.is_admin { return Err(ServiceError::Unauthorized); }
    Ok(())
}

fn update_category_check(lv: &u32, req: &CategoryUpdateRequest) -> QueryResult {
    check_admin_level(&req.category_name, &lv, 3)?;
    check_admin_level(&req.category_thumbnail, &lv, 3)
}

fn update_topic_check(lv: &u32, req: &TopicRequest) -> QueryResult {
    check_admin_level(&req.title, &lv, 3)?;
    check_admin_level(&req.category_id, &lv, 3)?;
    check_admin_level(&req.body, &lv, 3)?;
    check_admin_level(&req.thumbnail, &lv, 3)?;
    check_admin_level(&req.is_locked, &lv, 2)
}

fn update_post_check(lv: &u32, req: &PostRequest) -> QueryResult {
    check_admin_level(&req.topic_id, &lv, 3)?;
    check_admin_level(&req.post_id, &lv, 3)?;
    check_admin_level(&req.post_content, &lv, 3)?;
    check_admin_level(&req.is_locked, &lv, 2)
}

fn check_admin_level<T: Sized>(t: &Option<T>, self_admin_level: &u32, baseline_admin_level: u32) -> Result<(), ServiceError> {
    if let Some(_value) = t {
        if self_admin_level < &baseline_admin_level { return Err(ServiceError::Unauthorized); }
    }
    Ok(())
}
