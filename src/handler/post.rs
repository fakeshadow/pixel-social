use actix_web::{web, HttpResponse};
use chrono::Utc;
use diesel::prelude::*;

use crate::model::{
    errors::ServiceError,
    user::User,
    post::{Post, PostQuery, PostQueryResult, PostRequest},
    common::{PostgresPool, QueryOption, RedisPool, GlobalGuard},
};
use crate::schema::{posts, topics, users};
use crate::model::common::AttachUserRef;

type QueryResult = Result<HttpResponse, ServiceError>;

impl<'a> PostQuery<'a> {
    pub fn handle_query(self, opt: &QueryOption) -> QueryResult {
        let conn: &PgConnection = &opt.db_pool.unwrap().get().unwrap();
        match self {
            PostQuery::GetPost(post_id) => get_post(&post_id, &conn),
            PostQuery::AddPost(mut post_request) => add_post(&mut post_request, &opt.global_var, &conn),
            PostQuery::UpdatePost(post_request) => update_post(&post_request, &conn)
        }
    }
}

fn get_post(id: &u32, conn: &PgConnection) -> QueryResult {
    let post: Post = posts::table.find(&id).first::<Post>(conn)?;
    let user = users::table.find(&post.user_id).load::<User>(conn)?;
    Ok(PostQueryResult::GotPost(&post.to_ref().attach_user(&user)).to_response())
}

fn update_post(req: &PostRequest, conn: &PgConnection) -> QueryResult {
    let post_self_id = req.extract_self_id()?;
    // ToDo: get result from insert and pass it to redis
    match req.user_id {
        Some(_user_id) => diesel::update(posts::table
            .filter(posts::id.eq(&post_self_id).and(posts::user_id.eq(_user_id))))
            .set(req.make_update()?).execute(conn)?,
        None => diesel::update(posts::table
            .filter(posts::id.eq(&post_self_id)))
            .set(req.make_update()?).execute(conn)?
    };
    Ok(PostQueryResult::AddedPost.to_response())
}

fn add_post(req: &mut PostRequest, global_var: &Option<&web::Data<GlobalGuard>>, conn: &PgConnection) -> QueryResult {
    // ToDo: in case possible time region problem.
    let now = Utc::now().naive_local();
    let target_topic_id = req.extract_topic_id()?;

    let to_topic_check = diesel::update(topics::table
        .filter(topics::id.eq(&target_topic_id)))
        .set((topics::last_reply_time.eq(&now), topics::reply_count.eq(topics::reply_count + 1)))
        .execute(conn)?;
    if to_topic_check == 0 { return Err(ServiceError::NotFound); }

    if let Some(_post_id) = req.post_id {
        let to_post_check = diesel::update(posts::table
            .filter(posts::id.eq(&_post_id).and(posts::topic_id.eq(&target_topic_id))))
            .set((posts::last_reply_time.eq(&now), posts::reply_count.eq(posts::reply_count + 1)))
            .execute(conn)?;
        if to_post_check == 0 { req.post_id = None }
    }

    let id: u32 = global_var.unwrap().lock()
        .map(|mut guarded_global_var| guarded_global_var.next_pid())
        .map_err(|_| ServiceError::InternalServerError)?;

    // ToDo: get result from insert and pass it to redis
    diesel::insert_into(posts::table).values(&req.make_post(&id)?).execute(conn)?;
    Ok(PostQueryResult::AddedPost.to_response())
}
