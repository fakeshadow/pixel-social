use futures::future::IntoFuture;

use actix::prelude::*;

use crate::model::{
    actors::DatabaseService,
    user::{User, UpdateRequest},
    category::CategoryRequest,
    errors::ResError,
    post::PostRequest,
    topic::TopicRequest,
};

pub struct UpdateUserCheck(pub u32, pub UpdateRequest);

pub struct UpdateTopicCheck(pub u32, pub TopicRequest);

pub struct UpdatePostCheck(pub u32, pub PostRequest);

pub struct UpdateCategoryCheck(pub u32, pub CategoryRequest);

pub struct RemoveCategoryCheck(pub u32);


impl Message for UpdateUserCheck {
    type Result = Result<UpdateRequest, ResError>;
}

impl Message for UpdateTopicCheck {
    type Result = Result<TopicRequest, ResError>;
}

impl Message for UpdatePostCheck {
    type Result = Result<PostRequest, ResError>;
}

impl Message for UpdateCategoryCheck {
    type Result = Result<CategoryRequest, ResError>;
}

impl Message for RemoveCategoryCheck {
    type Result = Result<(), ResError>;
}

impl Handler<UpdateUserCheck> for DatabaseService {
    type Result = ResponseFuture<UpdateRequest, ResError>;

    fn handle(&mut self, msg: UpdateUserCheck, _: &mut Self::Context) -> Self::Result {
        let self_lv = msg.0;
        let req = msg.1;

        Box::new(Self::query_multi_limit(
            self.db.as_mut().unwrap(),
            self.users_by_id.as_ref().unwrap(),
            &[req.id.as_ref().unwrap()],
            self.error_reprot.as_ref().map(|e| e.clone()))
            .and_then(move |u: Vec<User>| {
                let u = u.first().ok_or(ResError::BadRequest)?;
                check_admin_level(&req.privilege, &self_lv, 9)?;
                if self_lv <= u.privilege { return Err(ResError::Unauthorized); }
                Ok(req)
            })
        )
    }
}

impl Handler<UpdateTopicCheck> for DatabaseService {
    type Result = ResponseFuture<TopicRequest, ResError>;

    fn handle(&mut self, msg: UpdateTopicCheck, _: &mut Self::Context) -> Self::Result {
        Box::new(update_topic_check(&msg.0, &msg.1)
            .into_future()
            .map(|_| msg.1))
    }
}

impl Handler<UpdateCategoryCheck> for DatabaseService {
    type Result = ResponseFuture<CategoryRequest, ResError>;

    fn handle(&mut self, msg: UpdateCategoryCheck, _: &mut Self::Context) -> Self::Result {
        Box::new(update_category_check(&msg.0, &msg.1)
            .into_future()
            .map(|_| msg.1))
    }
}

impl Handler<UpdatePostCheck> for DatabaseService {
    type Result = ResponseFuture<PostRequest, ResError>;

    fn handle(&mut self, msg: UpdatePostCheck, _: &mut Self::Context) -> Self::Result {
        Box::new(update_post_check(&msg.0, &msg.1)
            .into_future()
            .map(|_| msg.1))
    }
}

impl Handler<RemoveCategoryCheck> for DatabaseService {
    type Result = ResponseFuture<(), ResError>;

    fn handle(&mut self, msg: RemoveCategoryCheck, _: &mut Self::Context) -> Self::Result {
        Box::new(check_admin_level(&Some(1), &msg.0, 9)
            .into_future())
    }
}


type QueryResult = Result<(), ResError>;

fn update_category_check(lv: &u32, req: &CategoryRequest) -> QueryResult {
    check_admin_level(&req.name, &lv, 3)?;
    check_admin_level(&req.thumbnail, &lv, 3)
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

fn check_admin_level<T: Sized>(t: &Option<T>, self_admin_level: &u32, baseline_admin_level: u32) -> Result<(), ResError> {
    if let Some(_value) = t {
        if self_admin_level < &baseline_admin_level { return Err(ResError::Unauthorized); }
    }
    Ok(())
}
