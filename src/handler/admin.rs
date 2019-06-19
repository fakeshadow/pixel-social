use futures::future::{err as ft_err, IntoFuture};

use actix_web::web;
use diesel::prelude::*;

use crate::model::{
    category::CategoryUpdateRequest,
    common::{PoolConnectionPostgres, PostgresPool},
    errors::ServiceError,
    post::PostRequest,
    topic::TopicRequest,
    admin::AdminPrivilegeCheck,
};

type QueryResult = Result<(), ServiceError>;


impl<'a> AdminPrivilegeCheck<'a> {
    pub fn handle_check(self, db: &PostgresPool) -> QueryResult {
        let conn = &db.get().unwrap();
        match self {
            AdminPrivilegeCheck::UpdateCategoryCheck(lv, req) => update_category_check(&lv, &req),
            AdminPrivilegeCheck::UpdateTopicCheck(lv, req) => update_topic_check(&lv, &req),
            AdminPrivilegeCheck::UpdatePostCheck(lv, req) => update_post_check(&lv, &req),
            AdminPrivilegeCheck::DeleteCategoryCheck(lv) => if lv < &9 { Err(ServiceError::Unauthorized) } else { Ok(()) }
            _ => Ok(())
        }
    }
}


fn update_category_check(lv: &u32, req: &CategoryUpdateRequest) -> QueryResult {
    check_admin_level(&req.category_name, &lv, 3)?;
    check_admin_level(&req.category_thumbnail, &lv, 3)
}

fn update_topic_check(lv: &u32, req: &TopicRequest) -> QueryResult {
    check_admin_level(&req.title, &lv, 3)?;
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


use actix::prelude::*;

use crate::model::{
    actors::DatabaseService,
    user::{AuthRequest, AuthResponse, User, UpdateRequest},
};
use crate::handler::db::get_users;


pub struct UpdateUserCheck(pub u32, pub UpdateRequest);

pub struct UpdateTopicCheck(pub u32, pub TopicRequest);


impl Message for UpdateUserCheck {
    type Result = Result<UpdateRequest, ServiceError>;
}

impl Message for UpdateTopicCheck {
    type Result = Result<TopicRequest, ServiceError>;
}


impl Handler<UpdateUserCheck> for DatabaseService {
    type Result = ResponseFuture<UpdateRequest, ServiceError>;

    fn handle(&mut self, msg: UpdateUserCheck, ctx: &mut Self::Context) -> Self::Result {
        let self_lv = msg.0;
        let req = msg.1;

        Box::new(get_users(
            self.db.as_mut().unwrap(),
            self.users_by_id.as_ref().unwrap(),
            vec![req.id.unwrap()])
            .and_then(move |u: Vec<User>| {
                let u = u.first().ok_or(ServiceError::BadRequest)?;
                check_admin_level(&req.is_admin, &self_lv, 9)?;
                if self_lv <= u.is_admin { return Err(ServiceError::Unauthorized); }
                Ok(req)
            })
        )
    }
}

impl Handler<UpdateTopicCheck> for DatabaseService {
    type Result = ResponseFuture<TopicRequest, ServiceError>;

    fn handle(&mut self, msg: UpdateTopicCheck, ctx: &mut Self::Context) -> Self::Result {
        Box::new(update_topic_check(&msg.0, &msg.1)
            .into_future()
            .map(|_| msg.1))
    }
}