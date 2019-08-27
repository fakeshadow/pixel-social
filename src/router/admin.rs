use actix_web::{
    Error,
    HttpResponse, web::{Data, Json, Path},
};
use futures::{
    FutureExt,
    TryFutureExt,
};
use futures01::Future as Future01;

use crate::handler::{
    auth::UserJwt,
    cache::CacheService,
    db::DatabaseService,
};
use crate::model::{
    category::CategoryRequest,
    common::{
        GlobalVars,
        Validator,
    },
    errors::ResError,
    post::PostRequest,
    topic::TopicRequest,
    user::UpdateRequest,
};
use crate::handler::cache::{AddToCache, CheckCacheConn};

pub fn add_category(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    global: Data<GlobalVars>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> impl Future01<Item=HttpResponse, Error=Error> {
    add_category_async(jwt, req, global, cache, db).boxed_local().compat().from_err()
}

async fn add_category_async(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    global: Data<GlobalVars>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> Result<HttpResponse, ResError> {
    let req = req.into_inner().check_new()?;
    let c = db.admin_add_category(jwt.privilege, req, global.get_ref()).await?;

    let res = HttpResponse::Ok().json(&c);

    match cache.check_cache_conn().await {
        Ok(opt) => {
            actix::spawn(
                cache.if_replace_cache(opt)
                    .add_category_cache_01(&c)
                    .map_err(move |_| cache.send_failed_category(c))
            );
        }
        Err(_) => cache.send_failed_category(c)
    };

    Ok(res)
}

pub fn update_category(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> impl Future01<Item=HttpResponse, Error=Error> {
    update_category_async(jwt, req, cache, db).boxed_local().compat().from_err()
}

async fn update_category_async(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> Result<HttpResponse, ResError> {
    let req = req.into_inner().check_update()?;
    let c = db.admin_update_category(jwt.privilege, req).await?;

    let res = HttpResponse::Ok().json(&c);
    cache.update_categories(&[c]);

    Ok(res)
}

pub fn remove_category(
    jwt: UserJwt,
    id: Path<(u32)>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> impl Future01<Item=HttpResponse, Error=Error> {
    remove_category_async(jwt, id, cache, db).boxed_local().compat().from_err()
}

async fn remove_category_async(
    jwt: UserJwt,
    id: Path<(u32)>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> Result<HttpResponse, ResError> {
    let id = id.into_inner();

    let _ = db.admin_remove_category(id, jwt.privilege).await?;
    //ToDo: fix remove category cache
//    let _ = cache.remove_category(id).await?;

    Ok(HttpResponse::Ok().finish())
}


//pub fn update_user(
//    jwt: UserJwt,
//    req: Json<UpdateRequest>,
//    cache: Data<CacheService>,
//    db: Data<DatabaseService>,
//) -> impl Future<Item=HttpResponse, Error=Error> {
//    req.into_inner()
//        .attach_id(None)
//        .check_update()
//        .into_future()
//        .from_err()
//        .and_then(move |req| {
//            db.update_user_check(jwt.privilege, req)
//                .from_err()
//                .and_then(move |r| {
//                    db.update_user(r).from_err().and_then(move |u| {
//                        let res = HttpResponse::Ok().json(&u);
//                        cache.update_users(&[u]);
//                        res
//                    })
//                })
//        })
//}
//
//pub fn update_topic(
//    jwt: UserJwt,
//    req: Json<TopicRequest>,
//    cache: Data<CacheService>,
//    db: Data<DatabaseService>,
//) -> impl Future<Item=HttpResponse, Error=Error> {
//    req.into_inner()
//        .attach_user_id(None)
//        .check_update()
//        .into_future()
//        .from_err()
//        .and_then(move |req| {
//            db.admin_update_topic(jwt.privilege, &req)
//                .from_err()
//                .and_then(move |t| {
//                    let res = HttpResponse::Ok().json(&t);
//                    cache.update_topics(&[t]);
//                    res
//                })
//        })
//}
//
//pub fn update_post(
//    jwt: UserJwt,
//    req: Json<PostRequest>,
//    db: Data<DatabaseService>,
//    cache: Data<CacheService>,
//) -> impl Future<Item=HttpResponse, Error=Error> {
//    req.into_inner()
//        .attach_user_id(None)
//        .check_update()
//        .into_future()
//        .from_err()
//        .and_then(move |req| {
//            db.admin_update_post(jwt.privilege, req)
//                .from_err()
//                .and_then(move |p| {
//                    let res = HttpResponse::Ok().json(&p);
//                    cache.update_posts(&[p]);
//                    res
//                })
//        })
//}
