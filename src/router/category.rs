use actix::prelude::Future as Future01;
use actix_web::{
    web::{Data, Query},
    Error, HttpResponse,
};
use futures::future::{FutureExt, TryFutureExt};

use crate::{
    handler::{cache::CacheService, db::DatabaseService},
    model::{
        category::{CategoryQuery, QueryType},
        errors::ResError,
        topic::Topic,
    },
};

pub fn query_handler(
    req: Query<CategoryQuery>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    query_handler_async(req, db, cache).boxed_local().compat()
}

async fn query_handler_async(
    req: Query<CategoryQuery>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
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
                cache.update_categories(&c);
                Ok(HttpResponse::Ok().json(&c))
            }
        },
    }
}

async fn if_query_db(
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    result: Result<(Vec<Topic>, Vec<u32>), ResError>,
) -> Result<HttpResponse, Error> {
    let mut should_update_t = false;
    let mut should_update_u = false;

    let (t, uids) = match result {
        Ok(t) => t,
        Err(e) => {
            if let ResError::IdsFromCache(tids) = e {
                should_update_t = true;
                db.get_topics_with_uid(&tids).await?
            } else {
                return Err(e.into());
            }
        }
    };

    let u = match cache.get_users_from_ids(uids).await {
        Ok(u) => u,
        Err(e) => {
            if let ResError::IdsFromCache(uids) = e {
                should_update_u = true;
                db.get_users_by_id(&uids).await?
            } else {
                vec![]
            }
        }
    };

    if should_update_u {
        cache.update_users(&u);
    }
    if should_update_t {
        cache.update_topics(&t);
    }

    let res = Topic::attach_users(&t, &u);
    Ok(HttpResponse::Ok().json(&res))
}
