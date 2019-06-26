use std::fmt::Write;
use futures::{Future, future::{err as ft_err ,IntoFuture}};

use actix::prelude::*;
use chrono::Utc;

use crate::model::{
    actors::DatabaseService,
    errors::ServiceError,
    common::GlobalGuard,
    post::{Post, PostRequest},
};
use crate::handler::db::query_post_simple;

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

                let f = self.db
                    .as_mut()
                    .unwrap()
                    .query(self.insert_post.as_ref().unwrap(),
                           &[&id,
                               p.user_id.as_ref().unwrap(),
                               &p.topic_id.as_ref().unwrap(),
                               &p.category_id,
                               &p.post_id,
                               p.post_content.as_ref().unwrap(),
                               &now,
                               &now,
                               &now
                           ])
                    .into_future()
                    .map_err(|e| e.0)
                    .from_err()
                    .and_then(|(row, _)| {
                        match row {
                            Some(row) => Ok(Post {
                                id: row.get(0),
                                user_id: row.get(1),
                                topic_id: row.get(2),
                                category_id: row.get(3),
                                post_id: row.get(4),
                                post_content: row.get(5),
                                created_at: row.get(6),
                                updated_at: row.get(7),
                                last_reply_time: row.get(8),
                                reply_count: row.get(9),
                                is_locked: row.get(10),
                            }),
                            None => Err(ServiceError::BadRequest)
                        }
                    });

                Box::new(f)
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

                Box::new(query_post_simple(self.db.as_mut().unwrap(), &query))
            }
        }
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

        Box::new(query_post_simple(self.db.as_mut().unwrap(), &query)
            .map(|p| {
                let ids = vec![p.user_id];
                (vec![p], ids)
            }))
    }
}

