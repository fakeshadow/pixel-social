use actix_web::{
    web::{Data, Query},
    Error, HttpResponse,
};

use crate::handler::{
    auth::UserJwt,
    cache::pool_redis,
    db::pool,
    psn::{PSNRequest, PSNServiceAddr},
};

pub async fn query_handler(
    req: Query<PSNRequest>,
    addr: Data<PSNServiceAddr>,
) -> Result<HttpResponse, Error> {
    // send request to psn service no matter the local result.
    // psn service will handle if the request will add to psn queue by using time gate.
    let req_clone = req.clone();
    actix_rt::spawn(Box::pin(async move {
        let _ = addr.send(req_clone.into_msg(false)).await;
    }));

    // return local result if there is any.
    match &*req {
        PSNRequest::Profile { online_id } => {
            if let Ok(p) = pool_redis().get_psn_profile(online_id.as_str()).await {
                return Ok(HttpResponse::Ok().json(&p));
            }
        }
        PSNRequest::TrophyTitles { online_id, page } => {
            let page = page.parse::<u32>().unwrap_or(1);

            if let Ok(p) = pool_redis().get_psn_profile(online_id.as_str()).await {
                if let Ok(t) = pool().get_trophy_titles(p.np_id.as_str(), page).await {
                    return Ok(HttpResponse::Ok().json(&t));
                }
            }
        }
        PSNRequest::TrophySet {
            online_id,
            np_communication_id,
        } => {
            if let Ok(p) = pool_redis().get_psn_profile(online_id.as_str()).await {
                if let Ok(s) = pool()
                    .get_trophy_set(p.np_id.as_str(), np_communication_id.as_str())
                    .await
                {
                    return Ok(HttpResponse::Ok().json(&s));
                }
            }
        }
        _ => (),
    };

    Ok(HttpResponse::Ok().finish())
}

pub async fn query_handler_with_jwt(
    jwt: UserJwt,
    req: Query<PSNRequest>,
    addr: Data<PSNServiceAddr>,
) -> Result<HttpResponse, Error> {
    match *req {
        PSNRequest::Auth { .. } => {
            let req = req.into_inner().check_privilege(jwt.privilege)?;

            // auth request is add to the front of queue.
            actix_rt::spawn(async move {
                let _ = addr.send(req.into_msg(true)).await;
            });
        }
        PSNRequest::Activation { .. } => {
            actix_rt::spawn(async move {
                let _ = addr
                    .send(req.into_inner().attach_user_id(jwt.user_id).into_msg(false))
                    .await;
            });
        }
        _ => (),
    };
    Ok(HttpResponse::Ok().finish())
}

pub async fn community(// jwt_opt: Option<UserJwt>,
    //    req: Json<>,
) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().finish())
}
