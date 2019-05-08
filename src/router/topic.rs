use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, future::{Either, ok as ft_ok}, IntoFuture};

use crate::handler::{
    auth::UserJwt,
    user::get_users_async,
    cache::{UpdateCache, get_users_cache},
};
use crate::model::{
    common::{GlobalGuard, PostgresPool, RedisPool, AttachUser, Response},
    topic::{TopicRequest, TopicWithPost},
};

/// Async topic query. Only redis update is blocking.
pub fn add_topic(jwt: UserJwt, req: Json<TopicRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                 -> impl Future<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .attach_user_id_async(Some(jwt.user_id))
        .into_add_query()
        .into_topic(db, None)
        .from_err()
        .and_then(move |t| {
            let _ignore = UpdateCache::GotTopic(&t).handle_update(&Some(&cache));
            Response::ModifiedTopic.to_res()
        })
}

pub fn update_topic(jwt: UserJwt, req: Json<TopicRequest>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                    -> impl Future<Item=HttpResponse, Error=Error> {
    req.into_inner()
        .attach_user_id_async(Some(jwt.user_id))
        .into_update_query()
        .into_topic(db, None)
        .from_err()
        .and_then(move |t| {
            let _ignore = UpdateCache::GotTopic(&t).handle_update(&Some(&cache));
            Response::ModifiedTopic.to_res()
        })
}

pub fn get_topic(path: Path<(u32, i64)>, db: Data<PostgresPool>, cache: Data<RedisPool>)
                 -> impl Future<Item=HttpResponse, Error=Error> {
    use crate::model::{cache::PathToTopicQueryAsync, topic::PathToQueryAsync};
    path.to_query_cache()
        .topic_from_cache(cache.clone())
        .then(move |res| match res {
            Ok((t, p)) => Either::A(
                get_users_cache(&p, t.as_ref().map(|t| t.user_id), cache)
                    .from_err()
                    .and_then(move |u|
                        HttpResponse::Ok().json(&TopicWithPost::new(t.as_ref(), &p, &u)))),
            Err(_) => Either::B(
                path.to_query()
                    .into_topic_with_post(db.clone())
                    .from_err()
                    .and_then(move |(t, p)| {
                        if let Some(t) = &t {
                            let _ignore = UpdateCache::GotTopic(t).handle_update(&Some(&cache));
                        }
                        let _ignore = UpdateCache::GotPosts(&p).handle_update(&Some(&cache));
                        get_users_async(&p, t.as_ref().map(|t| t.user_id), db)
                            .from_err()
                            .and_then(move |u|
                                HttpResponse::Ok().json(&TopicWithPost::new(t.as_ref(), &p, &u)))
                    })
            )
        })
}