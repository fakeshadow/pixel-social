use std::fmt::Write;
use futures::future::err as ft_err;

use actix::prelude::*;
use chrono::Utc;

use crate::{
    CacheService,
    DatabaseService,
};

use crate::model::{
    errors::ResError,
    common::GlobalVars,
    post::{Post, PostRequest},
};

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

                Box::new(self
                    .insert_post(&[
                        &id,
                        p.user_id.as_ref().unwrap(),
                        &p.topic_id.as_ref().unwrap(),
                        &p.category_id,
                        &p.post_id,
                        p.post_content.as_ref().unwrap(),
                        now,
                        now
                    ]))
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
        Box::new(self.get_posts_by_id_with_uid(msg.0))
    }
}

#[derive(Message)]
pub struct AddPostCache(pub Post);

impl Handler<AddPostCache> for CacheService {
    type Result = ();

    fn handle(&mut self, msg: AddPostCache, ctx: &mut Self::Context) -> Self::Result {
        ctx.spawn(self
            .add_post_cache(msg.0)
            .into_actor(self));
    }
}


pub enum GetPostsCache {
    Old(u32, i64),
    Popular(u32, i64),
    Ids(Vec<u32>),
}

impl Message for GetPostsCache {
    type Result = Result<(Vec<Post>, Vec<u32>), ResError>;
}

impl Handler<GetPostsCache> for CacheService {
    type Result = ResponseFuture<(Vec<Post>, Vec<u32>), ResError>;

    fn handle(&mut self, msg: GetPostsCache, _: &mut Self::Context) -> Self::Result {
        match msg {
            GetPostsCache::Old(tid, page) => Box::new(self
                .get_cache_with_uids_from_zrange(&format!("topic:{}:posts_time", tid), page, "post")),
            GetPostsCache::Popular(tid, page) => Box::new(self
                .get_cache_with_uids_from_zrevrange_reverse_lex(&format!("topic:{}:posts_reply", tid), page, "post")),
            GetPostsCache::Ids(ids) => Box::new(self
                .get_cache_with_uids_from_ids(ids, "post"))
        }
    }
}