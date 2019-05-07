use actix_multipart::Multipart;
use actix_web::{error, Error, HttpResponse};
use futures::{Future, stream::Stream};

use crate::handler::{auth::UserJwt, stream::save_file};

pub fn upload_file(_: UserJwt, multipart: Multipart)
                   -> impl Future<Item=HttpResponse, Error=Error> {
    // ToDo: need to add an upload limit counter for user;
    multipart
        .map_err(error::ErrorInternalServerError)
        .map(|field| save_file(field).into_stream())
        .flatten()
        .collect()
        .map(|result| HttpResponse::Ok().json(result))
}
