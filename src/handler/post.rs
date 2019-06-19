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

pub struct GetPost(pub u32);


impl Message for ModifyPost {
    type Result = Result<Vec<Post>, ServiceError>;
}

impl Message for GetPost {
    type Result = Result<Vec<Post>, ServiceError>;
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

                let p = match msg.0.make_post(id) {
                    Ok(p) => p,
                    Err(e) => return Box::new(ft_err(e))
                };
                match p.post_id {
                    Some(to_pid) => format!("INSERT INTO posts
                            (id, user_id, topic_id, post_id, post_content)
                            VALUES ('{}', '{}', '{}', '{}', '{}')
                            RETURNING *", p.id, p.user_id, p.topic_id, to_pid, p.post_content),
                    None => format!("INSERT INTO posts
                            (id, user_id, topic_id, post_content)
                            VALUES ('{}', '{}', '{}', '{}')
                            RETURNING *", p.id, p.user_id, p.topic_id, p.post_content),
                }
            }
            None => {
                let p = match msg.0.make_update() {
                    Ok(p) => p,
                    Err(e) => return Box::new(ft_err(e))
                };
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
                    let _ = write!(&mut query, " updated_at = DEFAULT Where id='{}'", p.id);
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

impl Handler<GetPost> for DatabaseService {
    type Result = ResponseFuture<Vec<Post>, ServiceError>;

    fn handle(&mut self, msg: GetPost, _: &mut Self::Context) -> Self::Result {
        let query = format!("SELECT * FROM posts
        WHERE id='{}'", msg.0);

        Box::new(simple_query(
            self.db.as_mut().unwrap(),
            &query)
            .and_then(|msg| post_from_msg(&msg).map(|p| vec![p]))
        )
    }
}