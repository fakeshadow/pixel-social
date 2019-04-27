use actix_web::{web, HttpResponse};
use diesel::prelude::*;

use crate::model::{
    errors::ServiceError,
    post::Post,
    user::User,
    topic::{Topic, TopicWithPost, TopicQuery, TopicQueryResult, TopicRequest, TopicRef},
    common::{PostgresPool, QueryOption, GlobalGuard, AttachPublicUserRef, get_unique_id},
};
use crate::schema::{categories, posts, topics, users};

const LIMIT: i64 = 20;

type QueryResult = Result<HttpResponse, ServiceError>;

impl<'a> TopicQuery<'a> {
    pub fn handle_query(self, opt: &QueryOption) -> QueryResult {
        let conn: &PgConnection = &opt.db_pool.unwrap().get().unwrap();
        match self {
            TopicQuery::GetTopic(topic_id, page) => get_topic(&topic_id, &page, &conn),
            TopicQuery::AddTopic(new_topic_request) => add_topic(&new_topic_request, &opt.global_var, &conn),
            TopicQuery::UpdateTopic(topic_request) => update_topic(&topic_request, &conn)
        }
    }
}

fn get_topic(id: &u32, page: &i64, conn: &PgConnection) -> QueryResult {
    let offset = (page - 1) * 20;

    let topic: Topic = topics::table.filter(topics::id.eq(&id)).first::<Topic>(conn)?;
    let posts: Vec<Post> = posts::table
        .filter(posts::topic_id.eq(&id))
        .order(posts::id.asc()).limit(LIMIT).offset(offset).load::<Post>(conn)?;

    join_topics_users(topic.to_ref(), &posts, &conn, &page)
}

fn add_topic(req: &TopicRequest, global_var: &Option<&web::Data<GlobalGuard>>, conn: &PgConnection) -> QueryResult {
    // ToDo: increment category topic count instead of only checking.
    let category_check: usize = categories::table.find(&req.extract_category_id()?).execute(conn)?;
    if category_check == 0 { return Err(ServiceError::NotFound); };

    let id: u32 = global_var.unwrap().lock()
        .map(|mut guarded_global_var| guarded_global_var.next_tid())
        .map_err(|_| ServiceError::InternalServerError)?;

    diesel::insert_into(topics::table).values(&req.make_topic(&id)?).execute(conn)?;
    Ok(TopicQueryResult::ModifiedTopic.to_response())
}

fn update_topic(req: &TopicRequest, conn: &PgConnection) -> QueryResult {
    let topic_self_id = req.extract_self_id()?;

    match req.user_id {
        Some(_user_id) => diesel::update(topics::table
            .filter(topics::id.eq(&topic_self_id).and(topics::user_id.eq(_user_id))))
            .set(req.make_update()?).execute(conn)?,
        None => diesel::update(topics::table
            .filter(topics::id.eq(&topic_self_id)))
            .set(req.make_update()?).execute(conn)?
    };
    Ok(TopicQueryResult::ModifiedTopic.to_response())
}

fn join_topics_users(
    topic: TopicRef,
    posts: &Vec<Post>,
    conn: &PgConnection,
    page: &i64,
) -> QueryResult {
    let user_ids = get_unique_id(&posts, Some(topic.user_id));
    let users: Vec<User> = users::table.filter(users::id.eq_any(&user_ids)).load::<User>(conn)?;

    let _topic = topic.attach_user(&users);
    let posts = posts.iter().map(|post| post.to_ref().attach_user(&users)).collect();

    let result = if page == &1 {
        TopicWithPost::new(Some(&_topic), Some(&posts))
    } else {
        TopicWithPost::new(None, Some(&posts))
    };

    Ok(TopicQueryResult::GotTopic(&result).to_response())
}
