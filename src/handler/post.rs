use std::fmt::Write;
use futures::{Future, future::err as ft_err};

use actix::prelude::*;
use chrono::Utc;

use crate::model::{
    actors::DatabaseService,
    errors::ServiceError,
    common::GlobalGuard,
    post::{Post, PostRequest},
};
use crate::handler::db::{query_one, query_one_simple, query_multi_with_id};

pub struct ModifyPost(pub PostRequest, pub Option<GlobalGuard>);

pub struct GetPosts(pub Vec<u32>);

impl Message for ModifyPost {
    type Result = Result<Post, ServiceError>;
}

impl Message for GetPosts {
    type Result = Result<(Vec<Post>, Vec<u32>), ServiceError>;
}


impl Handler<ModifyPost> for DatabaseService {
    type Result = ResponseFuture<Post, ServiceError>;

    fn handle(&mut self, msg: ModifyPost, _: &mut Self::Context) -> Self::Result {
        match msg.1 {
            Some(g) => {
                let id = match g.lock() {
                    Ok(mut var) => var.next_pid(),
                    Err(_) => return Box::new(ft_err(ServiceError::InternalServerError))
                };

                let p = msg.0;
                let now = Utc::now().naive_local();

                Box::new(query_one(
                    self.db.as_mut().unwrap(),
                    self.insert_post.as_ref().unwrap(),
                    &[&id,
                        p.user_id.as_ref().unwrap(),
                        &p.topic_id.as_ref().unwrap(),
                        &p.category_id,
                        &p.post_id,
                        p.post_content.as_ref().unwrap(),
                        &now,
                        &now,
                        &now
                    ],
                ))
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
                    return Box::new(ft_err(ServiceError::BadRequest));
                }

                if let Some(s) = p.user_id {
                    let _ = write!(&mut query, " AND user_id = {}", s);
                }
                query.push_str(" RETURNING *");

                Box::new(query_one_simple(self.db.as_mut().unwrap(), &query))
            }
        }
    }
}

impl Handler<GetPosts> for DatabaseService {
    type Result = ResponseFuture<(Vec<Post>, Vec<u32>), ServiceError>;

    fn handle(&mut self, msg: GetPosts, _: &mut Self::Context) -> Self::Result {
        Box::new(
            query_multi_with_id(
                self.db.as_mut().unwrap(),
                self.posts_by_id.as_ref().unwrap(),
                &[&msg.0])
                .map(move |(mut t, uids): (Vec<Post>, Vec<u32>)| {
                    let mut result = Vec::with_capacity(t.len());
                    for i in 0..msg.0.len() {
                        for j in 0..t.len() {
                            if msg.0[i] == t[j].id {
                                result.push(t.swap_remove(j));
                                break;
                            }
                        }
                    }
                    (result, uids)
                }))
    }
}

