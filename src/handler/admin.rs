use futures::future::{err as ft_err, Either};

use actix::prelude::*;

use crate::handler::db::DatabaseService;
use crate::model::common::GlobalVars;
use crate::model::post::Post;
use crate::model::topic::Topic;
use crate::model::{
    category::{Category, CategoryRequest},
    errors::ResError,
    post::PostRequest,
    topic::TopicRequest,
    user::UpdateRequest,
};

impl DatabaseService {
    pub fn admin_update_topic(
        &self,
        self_level: u32,
        t: &TopicRequest,
    ) -> impl Future<Item = Topic, Error = ResError> {
        match update_topic_check(self_level, &t) {
            Ok(_) => Either::A(self.update_topic(t)),
            Err(e) => Either::B(ft_err(e)),
        }
    }

    pub fn admin_update_post(
        &self,
        self_level: u32,
        p: PostRequest,
    ) -> impl Future<Item = Post, Error = ResError> {
        match update_post_check(self_level, &p) {
            Ok(_) => Either::A(self.update_post(p)),
            Err(e) => Either::B(ft_err(e)),
        }
    }

    pub fn admin_add_category(
        &self,
        self_level: u32,
        req: CategoryRequest,
        g: &GlobalVars,
    ) -> impl Future<Item = Category, Error = ResError> {
        match update_category_check(self_level, &req) {
            Ok(_) => Either::A(self.add_category(req, g)),
            Err(e) => Either::B(ft_err(e)),
        }
    }

    pub fn admin_update_category(
        &self,
        self_level: u32,
        req: CategoryRequest,
    ) -> impl Future<Item = Category, Error = ResError> {
        match update_category_check(self_level, &req) {
            Ok(_) => Either::A(self.update_category(req)),
            Err(e) => Either::B(ft_err(e)),
        }
    }

    pub fn admin_remove_category(
        &self,
        cid: u32,
        self_level: u32,
    ) -> impl Future<Item = (), Error = ResError> {
        match check_admin_level(&Some(1), self_level, 9) {
            Ok(_) => Either::A(self.remove_category(cid)),
            Err(e) => Either::B(ft_err(e)),
        }
    }

    pub fn update_user_check(
        &self,
        self_level: u32,
        u: UpdateRequest,
    ) -> impl Future<Item = UpdateRequest, Error = ResError> {
        let id = vec![u.id.as_ref().copied().unwrap_or(0)];

        self.get_by_id::<crate::model::user::User>(&self.users_by_id, &id)
            .and_then(move |user| {
                let user = user.first().ok_or(ResError::BadRequest)?;
                check_admin_level(&u.privilege, self_level, 9)?;
                if self_level <= user.privilege {
                    return Err(ResError::Unauthorized);
                }
                Ok(u)
            })
    }
}

type QueryResult = Result<(), ResError>;

fn update_category_check(lv: u32, req: &CategoryRequest) -> QueryResult {
    check_admin_level(&req.name, lv, 3)?;
    check_admin_level(&req.thumbnail, lv, 3)
}

fn update_topic_check(lv: u32, req: &TopicRequest) -> QueryResult {
    check_admin_level(&req.title, lv, 3)?;
    check_admin_level(&req.body, lv, 3)?;
    check_admin_level(&req.thumbnail, lv, 3)?;
    check_admin_level(&req.is_locked, lv, 2)
}

fn update_post_check(lv: u32, req: &PostRequest) -> QueryResult {
    check_admin_level(&req.topic_id, lv, 3)?;
    check_admin_level(&req.post_id, lv, 3)?;
    check_admin_level(&req.post_content, lv, 3)?;
    check_admin_level(&req.is_locked, lv, 2)
}

fn check_admin_level<T: Sized>(
    t: &Option<T>,
    self_admin_level: u32,
    baseline_admin_level: u32,
) -> Result<(), ResError> {
    if let Some(_value) = t {
        if self_admin_level < baseline_admin_level {
            return Err(ResError::Unauthorized);
        }
    }
    Ok(())
}
