use futures::{
    future::{err as ft_err, Either},
    Future,
};
use std::fmt::Write;

use chrono::Utc;

use crate::handler::{cache::CacheService, db::DatabaseService};
use crate::model::{
    common::GlobalVars,
    errors::ResError,
    post::{Post, PostRequest},
};

impl DatabaseService {
    pub fn add_post(
        &self,
        p: PostRequest,
        g: &GlobalVars,
    ) -> impl Future<Item = Post, Error = ResError> {
        let id = match g.lock() {
            Ok(mut var) => var.next_pid(),
            Err(_) => return Either::A(ft_err(ResError::InternalServerError)),
        };

        let now = &Utc::now().naive_local();

        use crate::handler::db::Query;
        Either::B(self.query_one_trait(
            &self.insert_post,
            &[
                &id,
                p.user_id.as_ref().unwrap(),
                &p.topic_id.as_ref().unwrap(),
                &p.category_id,
                &p.post_id,
                p.post_content.as_ref().unwrap(),
                now,
                now,
            ],
        ))
    }

    pub fn update_post(&self, p: PostRequest) -> impl Future<Item = Post, Error = ResError> {
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

        if query.ends_with(',') {
            let _ = write!(
                &mut query,
                " updated_at = DEFAULT WHERE id = {}",
                p.id.unwrap()
            );
        } else {
            return Either::A(ft_err(ResError::BadRequest));
        }

        if let Some(s) = p.user_id {
            let _ = write!(&mut query, " AND user_id = {}", s);
        }
        query.push_str(" RETURNING *");

        use crate::handler::db::SimpleQuery;
        Either::B(self.simple_query_one_trait(query.as_str()))
    }
}

impl CacheService {
    pub fn get_posts_from_ids(
        &self,
        ids: Vec<u32>,
    ) -> impl Future<Item = (Vec<Post>, Vec<u32>), Error = ResError> {
        self.get_cache_with_uids_from_ids(ids, "post")
    }

    pub fn get_posts_old(
        &self,
        tid: u32,
        page: usize,
    ) -> impl Future<Item = (Vec<Post>, Vec<u32>), Error = ResError> {
        self.get_cache_with_uids_from_zrange(
            &format!("topic:{}:posts_time_created", tid),
            page,
            "post",
        )
    }

    pub fn get_posts_pop(
        &self,
        tid: u32,
        page: usize,
    ) -> impl Future<Item = (Vec<Post>, Vec<u32>), Error = ResError> {
        self.get_cache_with_uids_from_zrevrange_reverse_lex(
            &format!("topic:{}:posts_reply", tid),
            page,
            "post",
        )
    }
}
