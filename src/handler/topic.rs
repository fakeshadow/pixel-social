use actix_web::web;
use diesel::prelude::*;

use crate::model::{
    errors::ServiceError,
    post::Post,
    user::SlimUser,
    topic::{Topic, TopicWithPost, TopicQuery, TopicQueryResult, NewTopicRequest, TopicUpdateRequest},
    common::{PostgresPool, QueryOption, GlobalGuard},
};
use crate::schema::{categories, posts, topics, users};

const LIMIT: i64 = 20;

type CustomResult = Result<TopicQueryResult, ServiceError>;

impl<'a> TopicQuery<'a> {
    pub fn handle_query(self, opt: &QueryOption) -> CustomResult {
        let conn: &PgConnection = &opt.db_pool.unwrap().get().unwrap();
        match self {
            TopicQuery::GetTopic(topic_id, page) => get_topic(&topic_id, &page, &conn),
            TopicQuery::AddTopic(new_topic_request) => add_topic(&new_topic_request, &opt.global_var, &conn),
            TopicQuery::UpdateTopic(topic_request) => update_topic(&topic_request, &conn)
        }
    }
}

fn get_topic(topic_id: &u32, page: &i64, conn: &PgConnection) -> CustomResult {
    let offset = (page - 1) * 20;

    let _topic: Topic = topics::table
        .filter(topics::id.eq(&topic_id))
        .first::<Topic>(conn)?;

    let _posts: Vec<Post> = posts::table
        .filter(posts::topic_id.eq(&_topic.id))
        .order(posts::id.asc())
        .limit(LIMIT)
        .offset(offset)
        .load::<Post>(conn)?;

    join_topics_users(_topic, _posts, conn, &page)
}

fn add_topic(new_topic_request: &NewTopicRequest, global_var: &Option<&web::Data<GlobalGuard>>, conn: &PgConnection) -> CustomResult {
    let cid = new_topic_request.category_id;

    let category_check: usize = categories::table.find(&cid).execute(conn)?;
    if category_check == 0 { return Err(ServiceError::NotFound); };

    let id: u32 = global_var
        .unwrap()
        .lock()
        .map(|mut guarded_global_var| {
            let next_tid = guarded_global_var.next_tid;
            guarded_global_var.next_tid += 1;
            next_tid
        })
        .map_err(|_| ServiceError::InternalServerError)?;

    diesel::insert_into(topics::table)
        .values(&new_topic_request.attach_id(&id))
        .execute(conn)?;
    Ok(TopicQueryResult::AddedTopic)
}

fn update_topic(topic_request: &TopicUpdateRequest, conn: &PgConnection) -> CustomResult {
    let topic_self_id = topic_request.id;

    match topic_request.user_id {
        Some(_user_id) => {
            let topic_old_filter = topics::table.filter(
                topics::id.eq(&topic_self_id).and(topics::user_id.eq(_user_id)));

            diesel::update(topic_old_filter).set(topic_request).execute(conn)?;
        }
        None => {
            let topic_old_filter = topics::table.filter(
                topics::id.eq(&topic_self_id));

            diesel::update(topic_old_filter).set(topic_request).execute(conn)?;
        }
    };

    Ok(TopicQueryResult::AddedTopic)
}


fn join_topics_users(
    topic: Topic,
    posts: Vec<Post>,
    conn: &PgConnection,
    page: &i64,
) -> CustomResult {
    let select_user_columns = (
        users::id,
        users::username,
        users::email,
        users::avatar_url,
        users::signature,
        users::created_at,
        users::updated_at,
    );

    // use to bring the trait to scope
    use crate::model::common::MatchUser;
    let result = Post::get_unique_id(&posts, Some(topic.get_user_id()));

    let users: Vec<SlimUser> = users::table
        .filter(users::id.eq_any(&result))
        .select(&select_user_columns)
        .load::<SlimUser>(conn)?;

    let posts = posts
        .into_iter()
        .map(|post| post.attach_user(&users))
        .collect();
    let result = if page == &1 {
        TopicWithPost {
            topic: Some(topic.attach_user(&users)),
            posts: Some(posts),
        }
    } else {
        TopicWithPost {
            topic: None,
            posts: Some(posts),
        }
    };

    Ok(TopicQueryResult::GotTopicSlim(result))
}
