use actix::prelude::Future as Future01;
use actix_web::{
    web::{Data, Query},
    Error, HttpResponse,
};
use futures::future::{FutureExt, TryFutureExt};

use crate::handler::{cache::MyRedisPool, db::MyPostgresPool};
use crate::model::{
    category::{CategoryQuery, QueryType},
    errors::ResError,
    topic::Topic,
};

pub fn query_handler(
    req: Query<CategoryQuery>,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    query_handler_async(req, db, cache).boxed_local().compat()
}

async fn query_handler_async(
    req: Query<CategoryQuery>,
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
) -> Result<HttpResponse, Error> {
    match req.query_type {
        QueryType::Popular => {
            let result = cache
                .get_topics_pop(req.category_id.unwrap_or(1), req.page.unwrap_or(1))
                .await;

            if_query_db(db, cache, result).await
        }
        QueryType::PopularAll => {
            let result = cache.get_topics_pop_all(req.page.unwrap_or(1)).await;

            if_query_db(db, cache, result).await
        }
        QueryType::Latest => {
            let result = cache
                .get_topics_late(req.category_id.unwrap_or(1), req.page.unwrap_or(1))
                .await;

            if_query_db(db, cache, result).await
        }
        QueryType::All => match cache.get_categories_all().await {
            Ok(c) => Ok(HttpResponse::Ok().json(&c)),
            Err(_) => {
                let c = db.get_categories_all().await?;
                cache.update_categories(&c).await?;
                Ok(HttpResponse::Ok().json(&c))
            }
        },
    }
}

async fn if_query_db(
    db: Data<MyPostgresPool>,
    cache: Data<MyRedisPool>,
    result: Result<(Vec<Topic>, Vec<u32>), ResError>,
) -> Result<HttpResponse, Error> {
    let mut should_update_t = false;
    let mut should_update_u = false;

    let (t, uids) = match result {
        Ok(t) => t,
        Err(e) => {
            if let ResError::IdsFromCache(tids) = e {
                should_update_t = true;
                db.get_topics(&tids).await?
            } else {
                return Err(e.into());
            }
        }
    };

    let u = match cache.get_users(uids).await {
        Ok(u) => u,
        Err(e) => {
            if let ResError::IdsFromCache(uids) = e {
                should_update_u = true;
                db.get_users(&uids).await?
            } else {
                vec![]
            }
        }
    };

    if should_update_u {
        let _ = cache.update_users(&u).await;
    }
    if should_update_t {
        let _ = cache.update_topics(&t).await;
    }

    let res = Topic::attach_users(&t, &u);
    Ok(HttpResponse::Ok().json(&res))
}
