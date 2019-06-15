use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};
use futures::{Future, future::{Either, ok as ft_ok}};

use crate::handler::{
    auth::UserJwt,
    cache::{UpdateCacheAsync, get_unique_users_cache},
    user::get_unique_users};
use crate::model::{
    topic::TopicWithUser,
    cache::CacheQuery,
    category::{CategoryRequest, CategoryQuery},
    common::{PostgresPool, RedisPool, AttachUser},
};

pub fn get_all_categories(
    cache: Data<RedisPool>,
    db: Data<PostgresPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    CacheQuery::GetAllCategories
        .into_categories(&cache)
        .then(move |r| match r {
            Ok(c) => Either::A(ft_ok(HttpResponse::Ok().json(&c))),
            Err(_) => Either::B(CategoryQuery::GetAllCategories
                .into_categories(&db)
                .from_err()
                .and_then(move |c| {
                    let res = HttpResponse::Ok().json(&c);
                    UpdateCacheAsync::GotCategories(c).handler(&cache).then(|_| res)
                })
            )
        })
}

pub fn get_popular(page: Path<(i64)>, cache: Data<RedisPool>, db: Data<PostgresPool>)
                   -> impl Future<Item=HttpResponse, Error=Error> {
    // ToDo: Add get popular cache query
    ft_ok(HttpResponse::Ok().finish())
}

pub fn get_category(
    req: Path<(u32, i64)>,
    db: Data<PostgresPool>,
    cache: Data<RedisPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    use crate::model::{category::PathToQuery, cache::PathToTopicsQuery};

    req.to_query()
        .into_topics(&db)
        .from_err()
        .and_then(move |t|
            get_unique_users(&t, None, &db)
                .from_err()
                .and_then(move |u| HttpResponse::Ok().json(&t
                    .iter()
                    .map(|t| t.attach_user(&u))
                    .collect::<Vec<TopicWithUser>>())))


//    req.to_query_cache()
//        .into_topics(cache.get_ref().clone())
//        .then(move |r| match r {
//            Ok(t) => Either::A(
//                get_unique_users_cache(&t, None, cache.get_ref().clone())
//                    .from_err()
//                    .and_then(move |u|
//                        HttpResponse::Ok().json(&t
//                            .iter()
//                            .map(|t| t.attach_user(&u))
//                            .collect::<Vec<TopicWithUser>>()))),
//            Err(_) => Either::B(
//                req.to_query()
//                    .into_topics(&db)
//                    .from_err()
//                    .and_then(move |t|
//                        get_unique_users(&t, None, &db)
//                            .from_err()
//                            .and_then(move |u| {
//                                let res = HttpResponse::Ok().json(&t
//                                    .iter()
//                                    .map(|t| t.attach_user(&u))
//                                    .collect::<Vec<TopicWithUser>>());
//                                UpdateCacheAsync::GotTopics(t).handler(&cache).then(|_| res)
//                            })
//                    )
//            )
//        })
}

pub fn get_categories(
    req: Json<CategoryRequest>,
    db: Data<PostgresPool>,
    cache: Data<RedisPool>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    req.to_query()
        .into_topics(&db)
        .from_err()
        .and_then(move |t|
            get_unique_users(&t, None, &db)
                .from_err()
                .and_then(move |u| {
                    HttpResponse::Ok().json(&t
                        .iter()
                        .map(|t| t.attach_user(&u))
                        .collect::<Vec<TopicWithUser>>())
                }))
}
