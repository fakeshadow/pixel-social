use std::fmt::Write;
use futures::{future::err as ft_err};

use actix::prelude::*;
use chrono::Utc;

use crate::model::{
    actors::DatabaseService,
    errors::ResError,
    common::GlobalVars,
    post::{Post, PostRequest},
};
use crate::handler::db::SimpleQueryOne;

pub struct ModifyPost(pub PostRequest, pub Option<GlobalVars>);

pub struct GetPosts(pub Vec<u32>);

impl Message for ModifyPost {
    type Result = Result<Post, ResError>;
}

impl Message for GetPosts {
    type Result = Result<(Vec<Post>, Vec<u32>), ResError>;
}


impl Handler<ModifyPost> for DatabaseService {
    type Result = ResponseFuture<Post, ResError>;

    fn handle(&mut self, msg: ModifyPost, _: &mut Self::Context) -> Self::Result {
        match msg.1 {
            Some(g) => {
                let id = match g.lock() {
                    Ok(mut var) => var.next_pid(),
                    Err(_) => return Box::new(ft_err(ResError::InternalServerError))
                };

                let p = msg.0;
                let now = &Utc::now().naive_local();

                use crate::handler::db::QueryOne;
                Box::new(Self::query_one(
                    self.db.as_mut().unwrap(),
                    self.insert_post.as_ref().unwrap(),
                    &[&id,
                        p.user_id.as_ref().unwrap(),
                        &p.topic_id.as_ref().unwrap(),
                        &p.category_id,
                        &p.post_id,
                        p.post_content.as_ref().unwrap(),
                        now,
                        now],
                    self.error_reprot.as_ref().map(|r| r.clone())))
            }
            None => {
                let p = msg.0;

                let mut query = String::from("UPDATE posts SET");

                if let Some(s) = p.topic_id {
                    let _ = write!(&mut query, " topic_id = {},", s);
                }
                if let Some(s) = p.post_id {
                    let _ = write!(&mut query, " post_id = {},", s);
                }
                if let Some(s) = p.post_content {
                    let _ = write!(&mut query, " post_content = '{}',", s);
                }
                if let Some(s) = p.is_locked {
                    let _ = write!(&mut query, " is_locked = {},", s);
                }

                if query.ends_with(",") {
                    let _ = write!(&mut query, " updated_at = DEFAULT WHERE id = {}", p.id.unwrap());
                } else {
                    return Box::new(ft_err(ResError::BadRequest));
                }

                if let Some(s) = p.user_id {
                    let _ = write!(&mut query, " AND user_id = {}", s);
                }
                query.push_str(" RETURNING *");

                Box::new(self.simple_query_one(query.as_str()))
            }
        }
    }
}

impl Handler<GetPosts> for DatabaseService {
    type Result = ResponseFuture<(Vec<Post>, Vec<u32>), ResError>;

    fn handle(&mut self, msg: GetPosts, _: &mut Self::Context) -> Self::Result {
        use crate::handler::db::QueryMultiWithUids;
        Box::new(Self::query_multi_with_uid(
            self.db.as_mut().unwrap(),
            self.posts_by_id.as_ref().unwrap(),
            msg.0,
            self.error_reprot.as_ref().map(Clone::clone)))
    }
}

