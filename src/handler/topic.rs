use actix_web::{web, HttpResponse};
use diesel::prelude::*;

use crate::model::{
    errors::ServiceError,
    post::Post,
    user::User,
    topic::{Topic, TopicWithPost, TopicQuery, TopicQueryResult, TopicRequest, TopicRef},
    common::{PostgresPool, QueryOption, GlobalGuard, AttachUserRef, get_unique_id},
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

    let topic_raw: Topic = topics::table.filter(topics::id.eq(&id)).first::<Topic>(conn)?;
    let posts_raw: Vec<Post> = posts::table.filter(posts::topic_id.eq(&id)).order(posts::id.asc()).limit(LIMIT).offset(offset).load::<Post>(conn)?;
    let user_ids = get_unique_id(&posts_raw, Some(&topic_raw.user_id));
    let users: Vec<User> = users::table.filter(users::id.eq_any(&user_ids)).load::<User>(conn)?;
    /// update topic cache
    let _test = update_cache_test(&topic_raw.to_ref());

    let topic = topic_raw.to_ref().attach_user(&users);
    let posts = posts_raw.iter().map(|post| post.to_ref().attach_user(&users)).collect();
    let result = if page == &1 {
        TopicWithPost::new(Some(&topic), Some(&posts))
    } else {
        TopicWithPost::new(None, Some(&posts))
    };

    Ok(TopicQueryResult::GotTopic(&result).to_response())
}

use crate::handler::cache::*;

fn update_cache_test(topic: &TopicRef) -> Result<(), ServiceError> {
//    let vec = vec![topic];
    let hash = topic.to_hash();
    let rank = topic.to_rank();
    let vec = vec![rank];
    let result = serialize_vec(&vec)?;
    Ok(())
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
