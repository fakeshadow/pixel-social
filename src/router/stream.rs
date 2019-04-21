use rand::Rng;
use std::fs;
use std::io::Write;
use futures::{future, Future, IntoFuture, Stream};

use actix_web::{error, web, Error, HttpResponse};
use actix_multipart as multipart;

use crate::model::errors::ServiceError;
use crate::handler::auth::UserJwt;

#[derive(Serialize)]
pub struct UploadResponse {
    pub file_name: String,
    pub upload_name: String,
}

impl UploadResponse {
    fn new(file_name: &str, upload_name: String) -> UploadResponse {
        UploadResponse {
            file_name: file_name.to_string(),
            upload_name,
        }
    }
}

pub fn save_file(field: multipart::Field) -> Box<Future<Item = UploadResponse, Error = Error>> {
    // need to add an file size limiter here;

    let params = match field.content_disposition() {
        Some(content_disposition) => content_disposition,
        None => {
            return Box::new(future::err(error::ErrorBadRequest(
                "Form data key or content is empty",
            )));
        }
    };

    if params.parameters.len() < 2 {
        return Box::new(future::err(error::ErrorBadRequest("No file found")));
    }
    let file_name = params.parameters[1].as_filename().unwrap();

    match params.parameters[0].as_name() {
        Some(name) => {
            if name != "avatar" && name != "thumbnail" && name != "picture" {
                return Box::new(future::err(error::ErrorBadRequest(format!(
                    "{} is not a supported file type",
                    name
                ))));
            } else {
            }
        }
        None => return Box::new(future::err(error::ErrorBadRequest("No file type found"))),
    };

    let vec: Vec<&str> = file_name.rsplitn(2, ".").collect();
    if vec.len() < 2 {
        return Box::new(future::err(error::ErrorBadRequest(
            "No file extension found",
        )));
    }
    let origin_name = vec[1];
    let file_type = vec[0];

    if file_type != "jpg" && file_type != "png" && file_type != "gif" {
        return Box::new(future::err(error::ErrorBadRequest(format!(
            ".{} can't be uploaded",
            file_type
        ))));
    }

    let mut rng = rand::thread_rng();
    let random_number: u32 = rng.gen();

    let _file_name = format!("{}_{}.{}", origin_name, random_number, file_type);

    let mut file = match fs::File::create(format!("{}{}", "./public/", &_file_name)) {
        Ok(file) => file,
        Err(e) => return Box::new(future::err(error::ErrorInternalServerError(e))),
    };

    Box::new(
        field
            .fold(
                UploadResponse::new(file_name, _file_name),
                move |acc, bytes| {
                    let rt: Result<UploadResponse, multipart::MultipartError> = file
                        .write_all(bytes.as_ref())
                        .map(|_| acc)
                        .map_err(|e| multipart::MultipartError::Payload(error::PayloadError::Io(e)));
                    future::result(rt)
                },
            )
            .map_err(|e| error::ErrorInternalServerError(e)),
    )
}

pub fn handle_multipart_item(
    item: multipart::Field,
) -> Box<Stream<Item = UploadResponse, Error = Error>> {
    Box::new(save_file(item).into_stream())
}

pub fn upload_file(
    _: UserJwt,
    multipart: multipart::Multipart,
) -> impl Future<Item = HttpResponse, Error = ServiceError> {
    // need to add an upload limit counter for user;

    multipart
        .map(handle_multipart_item)
        .flatten()
        .collect()
        .map(|result| HttpResponse::Ok().json(result))
        .from_err()
}
