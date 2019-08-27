use actix_multipart::Multipart;
use actix_web::{error, Error, HttpResponse};
use futures::{
    FutureExt,
    TryFutureExt,
};
use futures01::{Future as Future01, stream::Stream};

use crate::handler::{auth::UserJwt, stream::save_file};

pub fn upload_file(
    _: UserJwt,
    multipart: Multipart,
) -> impl Future01<Item=HttpResponse, Error=Error> {
    multipart
        .map_err(error::ErrorInternalServerError)
        .map(|field|
            save_file(field)
                .boxed_local()
                .compat()
                .from_err()
                .into_stream()
        )
        .flatten()
        .collect()
        .map(|result| HttpResponse::Ok().json(result))
}
