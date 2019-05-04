use actix_web::HttpResponse;
use chrono::Utc;
use diesel::prelude::*;

use crate::schema::posts;
use crate::model::{
    errors::ServiceError,
    post::{Post, PostQuery, PostRequest},
    common::{Response, QueryOption, GlobalGuard, AttachUser, PoolConnectionPostgres},
};
use crate::handler::{
    user::get_user_by_id,
    topic::update_topic_reply_count,
    category::update_category_post_count,
    cache::UpdateCache,
};

const LIMIT: i64 = 20;

type QueryResult = Result<HttpResponse, ServiceError>;

impl<'a> PostQuery<'a> {
    pub fn handle_query(self, opt: &QueryOption) -> QueryResult {
        match self {
            PostQuery::GetPost(post_id) => get_post(&post_id, opt),
            PostQuery::AddPost(mut post_request) => add_post(&mut post_request, opt),
            PostQuery::UpdatePost(post_request) => update_post(&post_request, opt)
        }
    }
}

fn get_post(id: &u32, opt: &QueryOption) -> QueryResult {
    let conn = &opt.db_pool.unwrap().get().unwrap();
    let post: Post = posts::table.find(&id).first::<Post>(conn)?;
    let user = get_user_by_id(&post.user_id, conn)?;

    let _ignore = UpdateCache::GotPost(&post).handle_update(&opt.cache_pool);
    Ok(HttpResponse::Ok().json(&post.attach_user(&user)))
}

fn update_post(req: &PostRequest, opt: &QueryOption) -> QueryResult {
    let post_self_id = req.extract_self_id()?;
    let conn = &opt.db_pool.unwrap().get().unwrap();

    let post: Post = match req.user_id {
        Some(_user_id) => diesel::update(posts::table
            .filter(posts::id.eq(&post_self_id).and(posts::user_id.eq(_user_id))))
            .set(req.make_update()?).get_result(conn)?,
        None => diesel::update(posts::table
            .filter(posts::id.eq(&post_self_id)))
            .set(req.make_update()?).get_result(conn)?
    };

    let _ignore = UpdateCache::GotPost(&post).handle_update(&opt.cache_pool);
    Ok(Response::AddedPost.to_res())
}

fn add_post(req: &mut PostRequest, opt: &QueryOption) -> QueryResult {
    let topic_id = req.extract_topic_id()?;
    let conn = &opt.db_pool.unwrap().get().unwrap();
    // ToDo: in case possible time region problem.
    let now = Utc::now().naive_local();

    let post_old: Option<Post> = match req.post_id {
        Some(pid) => Some(diesel::update(posts::table
            .filter(posts::id.eq(&pid).and(posts::topic_id.eq(&topic_id))))
            .set((posts::last_reply_time.eq(&now), posts::reply_count.eq(posts::reply_count + 1)))
            .get_result(conn)?),
        None => None
    };
    let topic_update = update_topic_reply_count(topic_id, &now, conn)?;

    let id: u32 = opt.global_var.unwrap().lock()
        .map(|mut guarded_global_var| guarded_global_var.next_pid())
        .map_err(|_| ServiceError::InternalServerError)?;
    let post_new: Post = diesel::insert_into(posts::table).values(&req.make_post(&id, &now)?).get_result(conn)?;

    let category_update = update_category_post_count(&topic_update.category_id, conn)?;

    let _ignore = UpdateCache::AddedPost(&topic_update, &category_update, &post_new, &post_old).handle_update(&opt.cache_pool);
    Ok(Response::AddedPost.to_res())
}

pub fn get_posts_by_topic_id(id: &u32, offset: i64, conn: &PoolConnectionPostgres) -> Result<Vec<Post>, ServiceError> {
    Ok(posts::table.filter(posts::topic_id.eq(&id)).order(posts::id.asc()).limit(LIMIT).offset(offset).load::<Post>(conn)?)
}

pub fn load_all_posts_with_topic_id(conn: &PoolConnectionPostgres) -> Result<Vec<(u32, u32)>, ServiceError> {
    Ok(posts::table.select((posts::topic_id, posts::id)).order((posts::topic_id.asc(), posts::id.asc())).load(conn)?)
}

pub fn get_last_pid(conn: &PoolConnectionPostgres) -> Result<Vec<u32>, ServiceError> {
    Ok(posts::table.select(posts::id).order(posts::id.desc()).limit(1).load(conn)?)
}