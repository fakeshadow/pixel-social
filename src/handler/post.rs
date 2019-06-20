use std::fmt::Write;
use futures::{Future, future::err as ft_err};

use actix::prelude::*;

use crate::handler::{
    db::{simple_query, post_from_msg},
};
use crate::model::{
    actors::DatabaseService,
    errors::ServiceError,
    common::GlobalGuard,
    post::{Post, PostRequest},
};

const LIMIT: i64 = 20;

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

                match p.post_id {
                    Some(to_pid) => format!("INSERT INTO posts
                            (id, user_id, topic_id, post_id, post_content)
                            VALUES ('{}', '{}', '{}', '{}', '{}')
                            RETURNING *", id, p.user_id.unwrap(), p.topic_id.unwrap(), to_pid, p.post_content.unwrap()),
                    None => format!("INSERT INTO posts
                            (id, user_id, topic_id, post_content)
                            VALUES ('{}', '{}', '{}', '{}')
                            RETURNING *", id, p.user_id.unwrap(), p.topic_id.unwrap(), p.post_content.unwrap()),
                }
            }
            None => {
                let p = msg.0;

                let mut query = "UPDATE posts SET".to_owned();

                if let Some(s) = p.topic_id {
                    let _ = write!(&mut query, " topic_id='{}',", s);
                }
                if let Some(s) = p.post_id {
                    let _ = write!(&mut query, " post_id='{}',", s);
                }
                if let Some(s) = p.post_content {
                    let _ = write!(&mut query, " post_content='{}',", s);
                }
                if let Some(s) = p.is_locked {
                    let _ = write!(&mut query, " is_locked='{}',", s);
                }

                if query.ends_with(",") {
                    let _ = write!(&mut query, " updated_at = DEFAULT Where id='{}'", p.id.unwrap());
                } else {
                    return Box::new(ft_err(ServiceError::BadRequest));
                }

                if let Some(s) = p.user_id {
                    let _ = write!(&mut query, " AND user_id='{}'", s);
                }
                query.push_str(" RETURNING *");

                query
            }
        };

        Box::new(simple_query(
            self.db.as_mut().unwrap(),
            &query)
            .and_then(|msg| post_from_msg(&msg).map(|p| vec![p]))
        )
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

        Box::new(simple_query(
            self.db.as_mut().unwrap(),
            &query)
            .and_then(|msg| post_from_msg(&msg)
                .map(|p| {
                    let ids = vec![p.user_id];
                    (vec![p], ids)
                }))
        )
    }
}