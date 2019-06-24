use std::fmt::Write;
use futures::{Future, future::err as ft_err};

use actix::prelude::*;
use chrono::Utc;

use crate::handler::db::query_post;
use crate::model::{
    actors::DatabaseService,
    errors::ServiceError,
    common::GlobalGuard,
    post::{Post, PostRequest},
};

pub struct ModifyPost(pub PostRequest, pub Option<GlobalGuard>);

pub struct GetPosts(pub Vec<u32>);

impl Message for ModifyPost {
    type Result = Result<Vec<Post>, ServiceError>;
}

impl Message for GetPosts {
    type Result = Result<(Vec<Post>, Vec<u32>), ServiceError>;
}


impl Handler<ModifyPost> for DatabaseService {
    type Result = ResponseFuture<Vec<Post>, ServiceError>;

    fn handle(&mut self, msg: ModifyPost, _: &mut Self::Context) -> Self::Result {
        let query = match msg.1 {
            Some(g) => {
                let id = match g.lock() {
                    Ok(mut var) => var.next_pid(),
                    Err(_) => return Box::new(ft_err(ServiceError::InternalServerError))
                };

                let p = msg.0;

                let cid = p.category_id;
                let uid = p.user_id.unwrap();
                let tid = p.topic_id.unwrap();
                let content = p.post_content.unwrap();
                let now = Utc::now().naive_local();

                match p.post_id {
                    Some(to_pid) => {
                        format!("INSERT INTO posts
                            (id, user_id, topic_id, category_id, post_id, post_content, created_at)
                            VALUES ({}, {}, {}, {}, {}, '{}', '{}')
                            RETURNING *", id, uid, tid, cid, to_pid, &content, &now)
                    }
                    None => format!("INSERT INTO posts
                            (id, user_id, topic_id, category_id, post_content, created_at)
                            VALUES ({}, {}, {}, {}, '{}', '{}')
                            RETURNING *", id, uid, tid, cid, &content, &now),
                }
            }
            None => {
                let p = msg.0;

                let mut query = String::from("UPDATE posts SET");

                if let Some(s) = p.topic_id {
                    let _ = write!(&mut query, " topic_id={},", s);
                }
                if let Some(s) = p.post_id {
                    let _ = write!(&mut query, " post_id={},", s);
                }
                if let Some(s) = p.post_content {
                    let _ = write!(&mut query, " post_content='{}',", s);
                }
                if let Some(s) = p.is_locked {
                    let _ = write!(&mut query, " is_locked={},", s);
                }

                if query.ends_with(",") {
                    let _ = write!(&mut query, " updated_at = DEFAULT WHERE id={}", p.id.unwrap());
                } else {
                    return Box::new(ft_err(ServiceError::BadRequest));
                }

                if let Some(s) = p.user_id {
                    let _ = write!(&mut query, " AND user_id={}", s);
                }
                query.push_str(" RETURNING *");

                query
            }
        };

        Box::new(query_post(self.db.as_mut().unwrap(), &query).map(|p| vec![p]))
    }
}

impl Handler<GetPosts> for DatabaseService {
    type Result = ResponseFuture<(Vec<Post>, Vec<u32>), ServiceError>;

    fn handle(&mut self, msg: GetPosts, _: &mut Self::Context) -> Self::Result {
        let mut query = "SELECT * FROM posts
        WHERE id= ANY('{".to_owned();

        let len = msg.0.len();
        for (i, p) in msg.0.iter().enumerate() {
            if i < len - 1 {
                let _ = write!(&mut query, "{},", p);
            } else {
                let _ = write!(&mut query, "{}", p);
            }
        }
        query.push_str("}')");

        Box::new(query_post(self.db.as_mut().unwrap(), &query)
            .map(|p| {
                let ids = vec![p.user_id];
                (vec![p], ids)
            }))
    }
}

