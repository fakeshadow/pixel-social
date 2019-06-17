use futures::Future;

use actix_web::{HttpResponse, web::block};
use diesel::prelude::*;
use chrono::Utc;

use crate::handler::{
    category::update_category_post_count,
    topic::update_topic_reply_count,
};
use crate::model::{
    errors::ServiceError,
    common::{AttachUser, GlobalGuard, PoolConnectionPostgres, PostgresPool},
    topic::Topic,
    category::Category,
    post::{Post, PostQuery, PostRequest},
};
use crate::schema::posts;

const LIMIT: i64 = 20;

impl PostQuery {
    pub fn into_post(self, pool: PostgresPool) -> impl Future<Item=Post, Error=ServiceError> {
        block(move || match self {
            PostQuery::GetPost(id) => get_post(&id, &pool.get()?),
            PostQuery::UpdatePost(req) => update_post(&req, &pool.get()?),
            _ => panic!("method not allowed")
        }).from_err()
    }
    pub fn into_add_post(self, pool: PostgresPool, opt: Option<GlobalGuard>)
                         -> impl Future<Item=(Category, Topic, Option<Post>, Post), Error=ServiceError> {
        block(move || match self {
            PostQuery::AddPost(mut req) => add_post(&mut req, &pool.get()?, opt),
            _ => panic!("method not allowed")
        }).from_err()
    }
}

fn get_post(id: &u32, conn: &PoolConnectionPostgres) -> Result<Post, ServiceError> {
    Ok(posts::table.find(&id).first::<Post>(conn)?)
}

fn update_post(req: &PostRequest, conn: &PoolConnectionPostgres) -> Result<Post, ServiceError> {
    let post_self_id = req.extract_self_id()?;

    let post: Post = match req.user_id {
        Some(_user_id) => diesel::update(posts::table
            .filter(posts::id.eq(&post_self_id).and(posts::user_id.eq(_user_id))))
            .set(req.make_update()?).get_result(conn)?,
        None => diesel::update(posts::table
            .filter(posts::id.eq(&post_self_id)))
            .set(req.make_update()?).get_result(conn)?
    };
    Ok(post)
}

fn add_post(
    req: &mut PostRequest,
    conn: &PoolConnectionPostgres,
    global: Option<GlobalGuard>,
) -> Result<(Category, Topic, Option<Post>, Post), ServiceError> {
    let topic_id = req.extract_topic_id()?;

    // ToDo: in case possible time region problem.
    let now = Utc::now().naive_local();

    let post = match req.post_id {
        Some(pid) => Some(diesel::update(posts::table
            .filter(posts::id.eq(&pid).and(posts::topic_id.eq(&topic_id))))
            .set((posts::last_reply_time.eq(&now), posts::reply_count.eq(posts::reply_count + 1)))
            .get_result(conn)?),
        None => None
    };
    let topic = update_topic_reply_count(topic_id, &now, conn)?;

    let id: u32 = global.unwrap().lock()
        .map(|mut var| var.next_pid())
        .map_err(|_| ServiceError::InternalServerError)?;
    let post_new = diesel::insert_into(posts::table).values(&req.make_post(&id, &now)?).get_result(conn)?;
    let category = update_category_post_count(&topic.category_id, conn)?;

    Ok((category, topic, post, post_new))
}

/// helper query functions
pub fn get_posts_by_topic_id(id: &u32, offset: i64, conn: &PoolConnectionPostgres) -> Result<Vec<Post>, ServiceError> {
    Ok(posts::table.filter(posts::topic_id.eq(&id)).order(posts::id.asc()).limit(LIMIT).offset(offset).load::<Post>(conn)?)
}

pub fn load_all_posts_with_topic_id(conn: &PoolConnectionPostgres) -> Result<Vec<(u32, u32)>, ServiceError> {
    Ok(posts::table.select((posts::topic_id, posts::id)).order((posts::topic_id.asc(), posts::id.asc())).load(conn)?)
}

pub fn get_last_pid(conn: &PoolConnectionPostgres) -> u32 {
    posts::table.select(posts::id).order(posts::id.desc()).limit(1).first(conn).unwrap_or(1)
}