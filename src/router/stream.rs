use std::fs;
use std::io::Write;
use rand::Rng;

use actix_web::{dev, FutureResponse, HttpResponse, HttpRequest, multipart, Error, error, HttpMessage};
use futures::{Future, future, Stream};

use crate::app::AppState;
use crate::handler::auth::UserJwt;
use actix_web::error::{MultipartError};

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

pub fn save_file(
    field: multipart::Field<dev::Payload>, user_id: i32,
) -> Box<Future<Item=UploadResponse, Error=Error>> {

    // need to add an file size limiter here;

    let params = match field.content_disposition() {
        Some(content_disposition) => content_disposition,
        None => return Box::new(future::err(error::ErrorBadRequest("Form data key or content is empty")))
    };

    if params.parameters.len() < 2 { return Box::new(future::err(error::ErrorBadRequest("No file found"))); }
    let file_name = params.parameters[1].as_filename().unwrap();

    match params.parameters[0].as_name() {
        Some(name) => if name != "avatar" && name != "thumbnail" && name != "picture" {
            return Box::new(future::err(error::ErrorBadRequest(format!("{} is not a supported file type", name))));
        } else {},
        None => return Box::new(future::err(error::ErrorBadRequest("No file type found")))
    };

    let vec: Vec<&str> = file_name.split(".").collect();
    let origin_name = vec[0];
    let file_type = vec[vec.len() - 1];

    if file_type != "jpg" && file_type != "png" && file_type != "gif" {
        return Box::new(future::err(error::ErrorBadRequest(format!(".{} can't be uploaded", file_type))));
    }

    let mut rng = rand::thread_rng();
    let random_number: u32 = rng.gen();

    let _file_name = format!("{}_{}_{}.{}", &user_id, origin_name, random_number, file_type);

    let mut file = match fs::File::create(format!("{}{}", "./public/", &_file_name)) {
        Ok(file) => file,
        Err(e) => return Box::new(future::err(error::ErrorInternalServerError(e))),
    };

    Box::new(
        field
            .fold(UploadResponse::new(file_name, _file_name), move |acc, bytes| {
                let rt: Result<UploadResponse, MultipartError> = file
                    .write_all(bytes.as_ref())
                    .map(|_| acc)
                    .map_err(|e| {
                        error::MultipartError::Payload(error::PayloadError::Io(e))
                    });
                future::result(rt)
            })
            .map_err(|e| {
                error::ErrorInternalServerError(e)
            }),
    )
}

pub fn handle_multipart_item(
    item: multipart::MultipartItem<dev::Payload>, user_id: i32,
) -> Box<Stream<Item=UploadResponse, Error=Error>> {
    match item {
        multipart::MultipartItem::Field(field) => {
            Box::new(save_file(field, user_id).into_stream())
        }
        multipart::MultipartItem::Nested(mp) => Box::new(
            mp.map_err(error::ErrorInternalServerError)
                .map(move |item| { handle_multipart_item(item, user_id) })
                .flatten(),
        ),
    }
}

pub fn upload_file((user_jwt, req): (UserJwt, HttpRequest<AppState>)) -> FutureResponse<HttpResponse> {

    // need to add an upload limit counter for user;

    Box::new(req
        .multipart()
        .map(move |item| {
            handle_multipart_item(item, user_jwt.user_id)
        })
        .flatten()
        .collect()
        .map(|result| HttpResponse::Ok().json(result))
    )
}