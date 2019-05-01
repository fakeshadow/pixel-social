use actix_web::web;
use diesel::prelude::*;

use crate::model::{
    user::{User, UserUpdateRequest},
    category::CategoryUpdateRequest,
    topic::TopicRequest,
    post::PostRequest,
    admin::AdminQuery,
    errors::ServiceError,
    common::{PostgresPool, RedisPool, QueryOption},
};
use crate::schema::users;

type QueryResult = Result<(), ServiceError>;

impl<'a> AdminQuery<'a> {
    pub fn handle_query(self, opt: &QueryOption) -> QueryResult {
        let conn = &opt.db_pool.unwrap().get().unwrap();
        match self {
            AdminQuery::UpdateUserCheck(lv, req) => update_user_check(&lv, &req, &conn),
            AdminQuery::UpdateCategoryCheck(lv, req) => update_category_check(&lv, &req),
            AdminQuery::UpdateTopicCheck(lv, req) => update_topic_check(&lv, &req),
            AdminQuery::UpdatePostCheck(lv, req) => update_post_check(&lv, &req),
            AdminQuery::DeleteCategoryCheck(lv) => if lv < &9 { Err(ServiceError::Unauthorized) } else { Ok(()) }
        }
    }
}

fn update_user_check(lv: &u32, req: &UserUpdateRequest, conn: &PgConnection) -> QueryResult {
    check_admin_level(&req.is_admin, &lv, 9)?;
    let target_user: User = users::table.find(&req.id).first::<User>(conn)?;
    if lv <= &target_user.is_admin { return Err(ServiceError::Unauthorized); }
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
        if self_admin_level < &baseline_admin_level { return Err(ServiceError::Unauthorized) }
    }
    Ok(())
}
