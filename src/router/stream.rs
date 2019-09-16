use actix::prelude::{Future as Future01, Stream as Stream01};
use actix_multipart::Multipart;
use actix_web::{error, Error, HttpResponse};
use futures::{FutureExt, TryFutureExt};

use crate::handler::{auth::UserJwt, stream::save_file};

pub fn upload_file(
    _: UserJwt,
    multipart: Multipart,
) -> impl Future01<Item = HttpResponse, Error = Error> {
    multipart
        .map_err(error::ErrorInternalServerError)
        .map(|field| {
            save_file(field)
                .boxed_local()
                .compat()
                .from_err()
                .into_stream()
        })
        .flatten()
        .collect()
        .map(|r| HttpResponse::Ok().json(r))
}
