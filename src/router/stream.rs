use actix_multipart::Multipart;
use actix_web::{Error, HttpResponse};
use futures::TryStreamExt;

use crate::handler::{auth::UserJwt, stream::save_file};
// use crate::model::errors::ResError;

pub async fn upload_file(_: UserJwt, mut multipart: Multipart) -> Result<HttpResponse, Error> {
    // ToDo: move capacity limit to .env
    let mut result = Vec::with_capacity(5);

    // ignore bad field and keep iter the stream.
    // ToDo: add error info for failed multipart.
    while let Ok(Some(field)) = multipart.try_next().await {
            let r = save_file(field).await?;
            result.push(r);
    }

    Ok(HttpResponse::Ok().json(result))
}
