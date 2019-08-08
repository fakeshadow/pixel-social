use futures::{Future, IntoFuture};

use actix_web::{
    web::{Data, Json, Path},
    Error, HttpResponse,
};

use crate::handler::{auth::UserJwt, cache::CacheService, db::DatabaseService};
use crate::model::common::GlobalVars;
use crate::model::{
    category::CategoryRequest, common::Validator, post::PostRequest, topic::TopicRequest,
    user::UpdateRequest,
};

pub fn add_category(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    global: Data<GlobalVars>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let req = req.into_inner();
    req.check_new().into_future().from_err().and_then(move |_| {
        db.admin_add_category(jwt.privilege, req, global.get_ref())
            .from_err()
            .and_then(move |c| {
                let res = HttpResponse::Ok().json(&c);
                cache.add_category(c);
                res
            })
    })
}

pub fn update_category(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let req = req.into_inner();
    req.check_update()
        .into_future()
        .from_err()
        .and_then(move |_| {
            db.admin_update_category(jwt.privilege, req)
                .from_err()
                .and_then(move |c| {
                    let res = HttpResponse::Ok().json(&c);
                    cache.update_categories(vec![c]);
                    res
                })
        })
}

pub fn remove_category(
    jwt: UserJwt,
    id: Path<(u32)>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let id = id.into_inner();

    db.admin_remove_category(id, jwt.privilege)
        .from_err()
        .and_then(move |_| {
            cache
                .remove_category(id)
                .from_err()
                .and_then(|_| HttpResponse::Ok().finish())
        })
}

pub fn update_user(
    jwt: UserJwt,
    req: Json<UpdateRequest>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let req = req.into_inner().attach_id(None);
    req.check_update()
        .into_future()
        .from_err()
        .and_then(move |_| {
            db.update_user_check(jwt.privilege, req)
                .from_err()
                .and_then(move |r| {
                    db.update_user(r).from_err().and_then(move |u| {
                        let res = HttpResponse::Ok().json(&u);
                        cache.update_users(vec![u]);
                        res
                    })
                })
        })
}

pub fn update_topic(
    jwt: UserJwt,
    req: Json<TopicRequest>,
    cache: Data<CacheService>,
    db: Data<DatabaseService>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let mut req = req.into_inner().attach_user_id(None);
    req.check_update()
        .into_future()
        .from_err()
        .and_then(move |_| {
            db.admin_update_topic(jwt.privilege, req)
                .from_err()
                .and_then(move |t| {
                    let res = HttpResponse::Ok().json(&t);
                    cache.update_topics(vec![t]);
                    res
                })
        })
}

pub fn update_post(
    jwt: UserJwt,
    req: Json<PostRequest>,
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let mut req = req.into_inner().attach_user_id(None);
    req.check_update()
        .into_future()
        .from_err()
        .and_then(move |_| {
            db.admin_update_post(jwt.privilege, req)
                .from_err()
                .and_then(move |p| {
                    let res = HttpResponse::Ok().json(&p);
                    cache.update_posts(vec![p]);
                    res
                })
        })
}
