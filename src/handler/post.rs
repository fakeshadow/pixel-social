use actix_web::web;
use chrono::Utc;
use diesel::prelude::*;

use crate::model::{
    errors::ServiceError,
    post::{Post, PostQuery, PostQueryResult, PostRequest},
    common::{PostgresPool, QueryOption, RedisPool, GlobalGuard},
};
use crate::schema::{posts, topics};

type QueryResult = Result<PostQueryResult, ServiceError>;

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

fn get_post(pid: &u32, conn: &PgConnection) -> QueryResult {
    let post = posts::table.find(&pid).first::<Post>(conn)?;
    Ok(PostQueryResult::GotPost(post))
}

fn update_post(post_request: &PostRequest, conn: &PgConnection) -> QueryResult {
    let post_self_id = post_request.extract_self_id()?;

    match post_request.user_id {
        Some(_user_id) => {
            let post_old_filter = posts::table.filter(posts::id.eq(&post_self_id).and(posts::user_id.eq(_user_id)));
            diesel::update(post_old_filter).set(post_request.make_update()?).execute(conn)?;
        }
        None => {
            let post_old_filter = posts::table.filter(posts::id.eq(&post_self_id));
            diesel::update(post_old_filter).set(post_request.make_update()?).execute(conn)?;
        }
    };
    Ok(PostQueryResult::AddedPost)
}

fn add_post(post_request: &mut PostRequest, global_var: &Option<&web::Data<GlobalGuard>>, conn: &PgConnection) -> QueryResult {
    // ToDo: in case possible time region problem.
    let now = Utc::now().naive_local();
    let target_topic_id = post_request.extract_topic_id()?;

    let to_topic = topics::table.filter(topics::id.eq(&target_topic_id));
    let update_data = (
        topics::last_reply_time.eq(&now),
        topics::reply_count.eq(topics::reply_count + 1),
    );
    let to_topic_check = diesel::update(to_topic).set(update_data).execute(conn)?;
    if to_topic_check == 0 { return Err(ServiceError::NotFound); }

    if let Some(_post_id) = post_request.post_id {
        let to_post = posts::table.filter(posts::id.eq(&_post_id).and(posts::topic_id.eq(&target_topic_id)));
        let update_data = (posts::last_reply_time.eq(&now), posts::reply_count.eq(posts::reply_count + 1));

        let to_post_check = diesel::update(to_post).set(update_data).execute(conn)?;
        if to_post_check == 0 { post_request.post_id = None }
    }

    let id: u32 = global_var.unwrap().lock()
        .map(|mut guarded_global_var| guarded_global_var.next_pid())
        .map_err(|_| ServiceError::InternalServerError)?;

    diesel::insert_into(posts::table).values(&post_request.make_post(&id)?).execute(conn)?;
    Ok(PostQueryResult::AddedPost)
}
