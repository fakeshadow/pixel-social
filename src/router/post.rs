use actix_web::{
    web::{Data, Json, Path},
    Error, HttpResponse, ResponseError,
};
use futures::{compat::Future01CompatExt, FutureExt, TryFutureExt};
use futures01::Future as Future01;

use crate::handler::{
    auth::UserJwt,
    cache::{AddToCache, CacheService, CheckCacheConn},
    db::DatabaseService,
};
use crate::model::{
    common::GlobalVars,
    errors::ResError,
    post::{Post, PostRequest},
};

pub fn add(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Json<PostRequest>,
    global: Data<GlobalVars>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    add_async(jwt, db, cache, req, global)
        .boxed_local()
        .compat()
}

pub async fn add_async(
    jwt: UserJwt,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    req: Json<PostRequest>,
    global: Data<GlobalVars>,
) -> Result<HttpResponse, Error> {
    jwt.check_privilege()?;

    let req = req
        .into_inner()
        .attach_user_id(Some(jwt.user_id))
        .check_new()?;

    let p = db
        .check_conn()
        .await?
        .add_post(req, global.get_ref())
        .await?;

    let res = HttpResponse::Ok().json(&p);

    actix::spawn(
        async {
            match cache.check_cache_conn().await {
                Ok(opt) => {
                    let _ = cache
                        .if_replace_cache(opt)
                        .add_post_cache_01(&p)
                        .compat()
                        .map_err(move |_| cache.send_failed_post(p))
                        .await;
                }
                Err(_) => cache.send_failed_post(p),
            };
            Ok(())
        }
            .boxed_local()
            .compat(),
    );

    Ok(res)
}

pub fn update(
    jwt: UserJwt,
    req: Json<PostRequest>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    update_async(jwt, req, db, cache).boxed_local().compat()
}

async fn update_async(
    jwt: UserJwt,
    req: Json<PostRequest>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> Result<HttpResponse, Error> {
    let req = req
        .into_inner()
        .attach_user_id(Some(jwt.user_id))
        .check_update()?;

    let p = db.check_conn().await?.update_post(req).await?;

    let res = HttpResponse::Ok().json(&p);

    update_post_with_fail_check(cache, p);

    Ok(res)
}

pub fn update_post_with_fail_check(cache: Data<CacheService>, p: Post) {
    let p = vec![p];

    actix::spawn(
        async {
            match cache.check_cache_conn().await {
                Ok(opt) => {
                    let _ = cache
                        .if_replace_cache(opt)
                        .update_post_return_fail(p)
                        .map_err(move |p| cache.send_failed_post_update(p))
                        .await;
                }
                Err(_) => cache.send_failed_post_update(p),
            };
            Ok(())
        }
            .boxed_local()
            .compat(),
    );
}

pub fn get(
    id: Path<u32>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    get_async(id, db, cache).boxed_local().compat()
}

async fn get_async(
    id: Path<u32>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();

    let mut should_update_p = false;
    let mut should_update_u = false;

    let (p, uids) = match cache.get_posts_from_ids(vec![id]).await {
        Ok((p, uids)) => (p, uids),
        Err(e) => {
            if let ResError::IdsFromCache(pids) = e {
                should_update_p = true;
                db.get_posts_with_uid(&pids).await?
            } else {
                return Ok(e.render_response());
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
    if should_update_p {
        cache.update_posts(&p);
    }

    Ok(HttpResponse::Ok().json(Post::attach_users(&p, &u)))
}
