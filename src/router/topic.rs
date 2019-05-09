use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, future::Either};

use crate::handler::{auth::UserJwt, cache::UpdateCache};
use crate::model::{
    common::{GlobalGuard, PostgresPool, RedisPool, AttachUser, Response},
    topic::{TopicRequest, TopicWithPost},
};

/// Async topic query. Only redis update is blocking.
pub fn add_topic(
    jwt: UserJwt,
    req: Json<TopicRequest>,
    db: Data<PostgresPool>,
    cache: Data<RedisPool>,
    global: Data<GlobalGuard>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .attach_user_id_into(Some(jwt.user_id))
        .into_add_query()
        .into_topic_with_category(&db, Some(global))
        .from_err()
        .and_then(move |(c, t)| {
            let _ignore = UpdateCache::AddedTopic(&t, &c).handle_update(&Some(&cache));
            Response::ModifiedTopic.to_res()
        })
}

pub fn update_topic(jwt: UserJwt, req: Json<TopicRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                    -> impl Future<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .attach_user_id_into(Some(jwt.user_id))
        .into_update_query()
        .into_topic(&db, None)
        .from_err()
        .and_then(move |t| {
            let _ignore = UpdateCache::GotTopic(&t).handle_update(&Some(&cache));
            Response::ModifiedTopic.to_res()
        })
}

pub fn get_topic(path: Path<(u32, i64)>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                 -> impl Future<Item=HttpResponse, Error=Error> {
    use crate::model::{cache::PathToTopicQuery, topic::PathToQuery};
    use crate::handler::{user::get_unique_users, cache::get_unique_users_cache};

    path.to_query_cache()
        // ToDo: Break into_topic_with_post into two async query
        .into_topic_with_post(&cache)
        .then(move |res| match res {
            Ok((t, p)) => Either::A(
                get_unique_users_cache(&p, t.as_ref().map(|t| t.user_id), &cache)
                    .from_err()
                    .and_then(move |u|
                        HttpResponse::Ok().json(&TopicWithPost::new(t.as_ref(), &p, &u)))),
            Err(_) => Either::B(
                path.to_query()
                    .into_topic_with_post(&db)
                    .from_err()
                    .and_then(move |(t, p)| {
                        if let Some(t) = &t {
                            let _ignore = UpdateCache::GotTopic(t).handle_update(&Some(&cache));
                        }
                        let _ignore = UpdateCache::GotPosts(&p).handle_update(&Some(&cache));
                        get_unique_users(&p, t.as_ref().map(|t| t.user_id), &db)
                            .from_err()
                            .and_then(move |u|
                                HttpResponse::Ok().json(&TopicWithPost::new(t.as_ref(), &p, &u)))
                    })
            )
        })
}