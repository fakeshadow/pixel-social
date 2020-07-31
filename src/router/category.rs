use actix_web::{web::Query, Error, HttpResponse};

use crate::handler::{cache::MyRedisPool, data::DataRc, db::MyPostgresPool};
use crate::model::{
    category::{CategoryQuery, QueryType},
    errors::ResError,
    topic::Topic,
};

pub async fn query_handler(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    req: Query<CategoryQuery>,
) -> Result<HttpResponse, Error> {
    match req.query_type {
        QueryType::Popular => {
            let result = cache_pool
                .get_topics_pop(req.category_id.unwrap_or(1), req.page.unwrap_or(1))
                .await;

            if_query_db(db_pool, cache_pool, result).await
        }
        QueryType::PopularAll => {
            let result = cache_pool.get_topics_pop_all(req.page.unwrap_or(1)).await;

            if_query_db(db_pool, cache_pool, result).await
        }
        QueryType::Latest => {
            let result = cache_pool
                .get_topics_late(req.category_id.unwrap_or(1), req.page.unwrap_or(1))
                .await;

            if_query_db(db_pool, cache_pool, result).await
        }
        QueryType::All => match cache_pool.get_categories_all().await {
            Ok(c) => Ok(HttpResponse::Ok().json(&c)),
            Err(_) => {
                let c = db_pool.get_categories_all().await?;
                cache_pool.update_categories(&c).await?;
                Ok(HttpResponse::Ok().json(&c))
            }
        },
    }
}

async fn if_query_db(
    db_pool: DataRc<MyPostgresPool>,
    cache_pool: DataRc<MyRedisPool>,
    result: Result<(Vec<Topic>, Vec<u32>), ResError>,
) -> Result<HttpResponse, Error> {
    let mut should_update_t = false;
    let mut should_update_u = false;

    let (t, uids) = match result {
        Ok(t) => t,
        Err(e) => {
            if let ResError::IdsFromCache(tids) = e {
                should_update_t = true;
                db_pool.get_topics(&tids).await?
            } else {
                return Err(e.into());
            }
        }
    };

    let u = match cache_pool.get_users(uids).await {
        Ok(u) => u,
        Err(e) => {
            if let ResError::IdsFromCache(uids) = e {
                should_update_u = true;
                db_pool.get_users(&uids).await?
            } else {
                vec![]
            }
        }
    };

    let res = HttpResponse::Ok().json(&Topic::attach_users(&t, &u));

    actix_rt::spawn(async move {
        if should_update_u {
            let _ = cache_pool.update_users(&u).await;
        }
        if should_update_t {
            let _ = cache_pool.update_topics(&t).await;
        }
    });

    Ok(res)
}
