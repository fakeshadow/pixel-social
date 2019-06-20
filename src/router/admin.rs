use futures::{Future, future::Either, IntoFuture};

use actix_web::{HttpResponse, Error, web::{Data, Json, Path}};

use crate::model::{
    actors::{DB, CACHE},
    post::PostRequest,
    topic::TopicRequest,
    common::Validator,
    category::CategoryRequest,
    user::{ToUserRef, UpdateRequest},
};
use crate::handler::{
    auth::UserJwt,
    user::UpdateUser,
    category::{UpdateCategory, AddCategory, GetLastCategoryId},
    post::ModifyPost,
    topic::UpdateTopic,
    cache::UpdateCache,
    admin::{
        UpdatePostCheck,
        UpdateCategoryCheck,
        UpdateTopicCheck,
        UpdateUserCheck,
    },
};

pub fn add_category(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    cache: Data<CACHE>,
    db: Data<DB>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = req.into_inner();
    db.send(UpdateCategoryCheck(jwt.is_admin, req))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(move |req| db
            .send(GetLastCategoryId)
            .from_err()
            .and_then(|r| r)
            .from_err()
            .and_then(|cid| req.make_category(cid))
            .from_err()
            .and_then(move |req| db
                .send(AddCategory(req))
                .from_err()
                .and_then(|r| r)
                .from_err()
                .and_then(move |c| {
                    let res = HttpResponse::Ok().json(&c);
                    //ToDo: update category_id meta
                    let _ = cache.do_send(UpdateCache::Category(c));
                    res
                })
            ))
}

pub fn update_category(
    jwt: UserJwt,
    req: Json<CategoryRequest>,
    cache: Data<CACHE>,
    db: Data<DB>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = req.into_inner();
    db.send(UpdateCategoryCheck(jwt.is_admin, req))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(move |req| req.make_update())
        .from_err()
        .and_then(move |req| db
            .send(UpdateCategory(req))
            .from_err()
            .and_then(|r| r)
            .from_err()
            .and_then(move |c| {
                let res = HttpResponse::Ok().json(&c);
                let _ = cache.do_send(UpdateCache::Category(c));
                res
            })
        )
}

//pub fn admin_remove_category(
//    jwt: UserJwt, id: Path<(u32)>,
//    cache: Data<RedisPool>,
//    db: Data<PostgresPool>,
//) -> impl Future<Item=HttpResponse, Error=Error> {
//    // ToDo: need to add posts and topics migration along side the remove.
//    use crate::model::{admin::IdToQuery as AdminIdToQuery, category::IdToQuery};
//    id.to_privilege_check(&jwt.is_admin)
//        .handle_check(&db)
//        .into_future()
//        .from_err()
//        .and_then(move |_| id
//            .to_delete_query()
//            .into_category_id(&db))
//        .from_err()
//        .and_then(move |id| HttpResponse::Ok().json(id))
//}

pub fn update_user(
    jwt: UserJwt,
    req: Json<UpdateRequest>,
    cache: Data<CACHE>,
    db: Data<DB>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = req.into_inner().attach_id(None);
    req.check_update()
        .into_future()
        .from_err()
        .and_then(move |_| db
            .send(UpdateUserCheck(jwt.is_admin, req))
            .from_err()
            .and_then(|r| r)
            .from_err()
            .and_then(move |r| db
                .send(UpdateUser(r))
                .from_err()
                .and_then(|r| r)
                .from_err()
                .and_then(move |u| {
                    let res = HttpResponse::Ok().json(&u);
                    let _ = cache.do_send(UpdateCache::User(u));
                    res
                })))
}

pub fn update_topic(
    jwt: UserJwt,
    req: Json<TopicRequest>,
    cache: Data<CACHE>,
    db: Data<DB>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = req.into_inner().attach_user_id(None);
    db.send(UpdateTopicCheck(jwt.is_admin, req))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(move |r| db
            .send(UpdateTopic(r))
            .from_err()
            .and_then(|r| r)
            .from_err()
            .and_then(move |t| {
                let res = HttpResponse::Ok().json(&t);
                let _ = cache.do_send(UpdateCache::Topic(t));
                res
            }))
}

pub fn admin_update_post(
    jwt: UserJwt,
    req: Json<PostRequest>,
    db: Data<DB>,
    cache: Data<CACHE>,
) -> impl Future<Item=HttpResponse, Error=Error> {
    let req = req.into_inner().attach_user_id(None);
    db.send(UpdatePostCheck(jwt.is_admin, req))
        .from_err()
        .and_then(|r| r)
        .from_err()
        .and_then(move |r| db
            .send(ModifyPost(r, None))
            .from_err()
            .and_then(|r| r)
            .from_err()
            .and_then(move |p| {
                let res = HttpResponse::Ok().json(&p);
                let _ = cache.do_send(UpdateCache::Post(p));
                res
            }))
}
