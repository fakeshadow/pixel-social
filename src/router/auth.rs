use actix_web::{
    Error,
    HttpResponse, ResponseError, web::{Data, Json, Path},
};
use futures::{
    FutureExt,
    TryFutureExt,
};
use futures01::Future as Future01;

use crate::handler::{auth::UserJwt, cache::CacheService, db::DatabaseService};
use crate::model::{
    common::{GlobalVars, Validator},
    errors::ResError,
    user::{AuthRequest, UpdateRequest, User},
};

pub fn login(
    db: Data<DatabaseService>,
    req: Json<AuthRequest>,
) -> impl Future01<Item=HttpResponse, Error=Error> {
    login_async(db, req).boxed_local().compat().from_err()
}

async fn login_async(
    db: Data<DatabaseService>,
    req: Json<AuthRequest>,
) -> Result<HttpResponse, ResError> {
    let r = req.into_inner().check_login()?;
    let r = db.login(r).await?;
    Ok(HttpResponse::Ok().json(&r))
}

pub fn register(
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    global: Data<GlobalVars>,
    req: Json<AuthRequest>,
) -> impl Future01<Item=HttpResponse, Error=Error> {
    register_async(db, cache, global, req).boxed_local().compat().from_err()
}

async fn register_async(
    db: Data<DatabaseService>,
    cache: Data<CacheService>,
    global: Data<GlobalVars>,
    req: Json<AuthRequest>,
) -> Result<HttpResponse, ResError> {
    let req = req.into_inner().check_register()?;

    let opt = db.check_conn().await?;

    let _ = db.if_replace_db(opt).check_register(&req).await?;

    let u = db.register(req, global.get_ref()).await?;

    let res = HttpResponse::Ok().json(&u);

    cache.add_activation_mail(u.clone());

    actix::spawn(
        cache.update_user_return_fail(vec![u])
            .map_err(move |u| cache.send_failed_user(u))
    );

    Ok(res)
}

//pub fn activate_by_mail(
//    db: Data<DatabaseService>,
//    cache: Data<CacheService>,
//    req: Path<(String)>,
//) -> impl Future<Item = HttpResponse, Error = Error> {
//    let uuid = req.into_inner();
//
//    cache
//        .get_uid_from_uuid(uuid.as_str())
//        .from_err()
//        .and_then(move |uid| {
//            db.update_user(UpdateRequest::make_active(uid))
//                .from_err()
//                .and_then(move |u| {
//                    // request for another login after the activation to update user's jwt token.
//                    let res = HttpResponse::Ok().json(&u);
//                    cache.update_users(&[u]);
//                    cache.remove_activation_uuid(uuid.as_str());
//                    res
//                })
//        })
//}
//
//pub fn add_activation_mail(
//    jwt: UserJwt,
//    db: Data<DatabaseService>,
//    cache: Data<CacheService>,
//) -> impl Future<Item = HttpResponse, Error = Error> {
//    cache
//        .get_users_from_ids(vec![jwt.user_id])
//        .then(move |r| match r {
//            Ok(u) => Either::A(ft_ok(pop_user_add_activation_mail(cache, u))),
//            Err(e) => Either::B(match e {
//                ResError::IdsFromCache(ids) => Either::A(
//                    db.get_users_by_id(&ids)
//                        .from_err()
//                        .and_then(|u| pop_user_add_activation_mail(cache, u)),
//                ),
//                _ => Either::B(ft_ok(e.render_response())),
//            }),
//        })
//}
//
//fn pop_user_add_activation_mail(cache: Data<CacheService>, mut u: Vec<User>) -> HttpResponse {
//    match u.pop() {
//        Some(u) => {
//            cache.add_activation_mail(u);
//            HttpResponse::Ok().finish()
//        }
//        None => ResError::BadRequest.render_response(),
//    }
//}
