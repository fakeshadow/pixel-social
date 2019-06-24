use std::fmt::Write;
use futures::{Future, future::err as ft_err};

use actix::prelude::*;

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

                match p.post_id {
                    Some(to_pid) => {
                        format!("INSERT INTO posts{}
                            (id, user_id, topic_id,category_id, post_id, post_content)
                            VALUES ({}, {}, {}, {}, {}, '{}')
                            RETURNING *", cid, id, uid, tid, cid, to_pid, &content)
                    }
                    None => format!("INSERT INTO posts{}
                            (id, user_id, topic_id,category_id, post_content)
                            VALUES ({}, {}, {}, {}, '{}')
                            RETURNING *", cid, id, uid, tid, cid, &content),
                }
            }
            None => {
                let p = msg.0;

                let mut query = format!("UPDATE posts{} SET", p.category_id);

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
                    let _ = write!(&mut query, " updated_at = DEFAULT Where id={}", p.id.unwrap());
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

