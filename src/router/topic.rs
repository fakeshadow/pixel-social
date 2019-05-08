use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, future::result as ftr, IntoFuture};

use crate::handler::{auth::UserJwt, cache::handle_cache_query, user::get_unique_users};
use crate::model::{
    common::{GlobalGuard, PostgresPool, QueryOption, QueryOptAsync, RedisPool, AttachUser, Response},
    errors::ServiceError,
    topic::{TopicRequest, TopicWithPost},
};
use crate::handler::cache::UpdateCache;
use crate::handler::user_async::{get_unique_users_async, get_user_by_id_async};

pub fn add_topic(jwt: UserJwt, mut req: Json<TopicRequest>, global: Data<GlobalGuard>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                 -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    req.attach_user_id(Some(jwt.user_id))
        .to_add_query()
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), Some(&global)))
        .into_future()
}

pub fn update_topic(jwt: UserJwt, mut req: Json<TopicRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                    -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    req.attach_user_id(Some(jwt.user_id))
        .to_update_query()
        .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
        .into_future()
}

pub fn get_topic(path: Path<(u32, i64)>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                 -> impl IntoFuture<Item=HttpResponse, Error=ServiceError> {
    use crate::model::{cache::IdToTopicQuery, topic::IdToQuery};
    let (id, page) = path.into_inner();
    handle_cache_query(id.to_query_cache(&page), &cache)
        .into_future()
        .then(move |res| match res {
            Ok(res) => ftr(Ok(res)),
            Err(_) => id
                .into_query(page)
                .handle_query(&QueryOption::new(Some(&db), Some(&cache), None))
                .into_future()
        })
}


/// Async topic query. Only redis update is blocking.

pub fn add_topic_async(jwt: UserJwt, req: Json<TopicRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                       -> impl IntoFuture<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .attach_user_id_async(Some(jwt.user_id))
        .into_add_query_async()
        .into_topic(QueryOptAsync::new(Some(db.clone()), None))
        .from_err()
        .and_then(move |t| {
            let _ignore = UpdateCache::GotTopic(&t).handle_update(&Some(&cache));
            Response::ModifiedTopic.to_res()
        })
}

pub fn update_topic_async(jwt: UserJwt, req: Json<TopicRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                          -> impl IntoFuture<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .attach_user_id_async(Some(jwt.user_id))
        .into_update_query_async()
        .into_topic(QueryOptAsync::new(Some(db.clone()), None))
        .from_err()
        .and_then(move |t| {
            let _ignore = UpdateCache::GotTopic(&t).handle_update(&Some(&cache));
            get_user_by_id_async(t.id, db.get().unwrap())
                .from_err()
                .and_then(move |u| Ok(HttpResponse::Ok().json(t.attach_user(&u))))
        })
}

pub fn get_topic_async(path: Path<(u32, i64)>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                       -> impl IntoFuture<Item=HttpResponse, Error=Error> {
    use crate::model::topic::IdToQueryAsync;

    let (id, page) = path.into_inner();
    id.into_query(page)
        .into_topic_with_post(QueryOptAsync::new(Some(db.clone()), None))
        .from_err()
        .and_then(move |(topic, posts)| {
            if let Some(t) = &topic {
                let _ignore = UpdateCache::GotTopic(&t).handle_update(&Some(&cache));
            }
            let _ignore = UpdateCache::GotPosts(&posts).handle_update(&Some(&cache));
            get_unique_users_async(&posts, topic.as_ref().map(|t| t.id), db.get().unwrap())
                .from_err()
                .and_then(move |u| Ok(HttpResponse::Ok().json(&TopicWithPost::new(
                    topic.as_ref().map(|t| t.attach_user(&u)),
                    Some(posts.iter().map(|post| post.attach_user(&u)).collect())))))
        })
}