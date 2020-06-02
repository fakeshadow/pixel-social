use actix_web::{web::Query, Error, HttpResponse};

use crate::handler::{cache::pool_redis, db::pool};
use crate::model::{
    category::{CategoryQuery, QueryType},
    errors::ResError,
    topic::Topic,
};

pub async fn query_handler(req: Query<CategoryQuery>) -> Result<HttpResponse, Error> {
    match req.query_type {
        QueryType::Popular => {
            let result = pool_redis()
                .get_topics_pop(req.category_id.unwrap_or(1), req.page.unwrap_or(1))
                .await;

            if_query_db(result).await
        }
        QueryType::PopularAll => {
            let result = pool_redis().get_topics_pop_all(req.page.unwrap_or(1)).await;

            if_query_db(result).await
        }
        QueryType::Latest => {
            let result = pool_redis()
                .get_topics_late(req.category_id.unwrap_or(1), req.page.unwrap_or(1))
                .await;

            if_query_db(result).await
        }
        QueryType::All => match pool_redis().get_categories_all().await {
            Ok(c) => Ok(HttpResponse::Ok().json(&c)),
            Err(_) => {
                let c = pool().get_categories_all().await?;
                pool_redis().update_categories(&c).await?;
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
                pool().get_topics(&tids).await?
            } else {
                return Err(e.into());
            }
        }
    };

    let u = match pool_redis().get_users(uids).await {
        Ok(u) => u,
        Err(e) => {
            if let ResError::IdsFromCache(uids) = e {
                should_update_u = true;
                pool().get_users(&uids).await?
            } else {
                vec![]
            }
        }
    };

    if should_update_u {
        let _ = pool_redis().update_users(&u).await;
    }
    if should_update_t {
        let _ = pool_redis().update_topics(&t).await;
    }

    let res = Topic::attach_users(&t, &u);
    Ok(HttpResponse::Ok().json(&res))
}
