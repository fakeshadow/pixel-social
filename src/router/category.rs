use actix_web::{web::Query, Error, HttpResponse};

use crate::handler::{cache::POOL_REDIS, db::POOL};
use crate::model::{
    category::{CategoryQuery, QueryType},
    errors::ResError,
    topic::Topic,
};

pub async fn query_handler(req: Query<CategoryQuery>) -> Result<HttpResponse, Error> {
    match req.query_type {
        QueryType::Popular => {
            let result = POOL_REDIS
                .get_topics_pop(req.category_id.unwrap_or(1), req.page.unwrap_or(1))
                .await;

            if_query_db(result).await
        }
        QueryType::PopularAll => {
            let result = POOL_REDIS.get_topics_pop_all(req.page.unwrap_or(1)).await;

            if_query_db(result).await
        }
        QueryType::Latest => {
            let result = POOL_REDIS
                .get_topics_late(req.category_id.unwrap_or(1), req.page.unwrap_or(1))
                .await;

            if_query_db(result).await
        }
        QueryType::All => match POOL_REDIS.get_categories_all().await {
            Ok(c) => Ok(HttpResponse::Ok().json(&c)),
            Err(_) => {
                let c = POOL.get_categories_all().await?;
                POOL_REDIS.update_categories(&c).await?;
                Ok(HttpResponse::Ok().json(&c))
            }
        },
    }
}

async fn if_query_db(
    result: Result<(Vec<Topic>, Vec<u32>), ResError>,
) -> Result<HttpResponse, Error> {
    let mut should_update_t = false;
    let mut should_update_u = false;

    let (t, uids) = match result {
        Ok(t) => t,
        Err(e) => {
            if let ResError::IdsFromCache(tids) = e {
                should_update_t = true;
                POOL.get_topics(&tids).await?
            } else {
                return Err(e.into());
            }
        }
    };

    let u = match POOL_REDIS.get_users(uids).await {
        Ok(u) => u,
        Err(e) => {
            if let ResError::IdsFromCache(uids) = e {
                should_update_u = true;
                POOL.get_users(&uids).await?
            } else {
                vec![]
            }
        }
    };

    if should_update_u {
        let _ = POOL_REDIS.update_users(&u).await;
    }
    if should_update_t {
        let _ = POOL_REDIS.update_topics(&t).await;
    }

    let res = Topic::attach_users(&t, &u);
    Ok(HttpResponse::Ok().json(&res))
}
