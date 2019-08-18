use std::fs;
use std::io::Write;

use actix_multipart::{Field, MultipartError};
use actix_web::{error, web, Error};
use futures::{
    future::{err, Either},
    Future, Stream,
};
use rand::Rng;

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

pub fn save_file(field: Field) -> impl Future<Item = UploadResponse, Error = Error> {
    // need to add an file size limiter here;

    let params = match field.content_disposition() {
        Some(content_disposition) => content_disposition,
        None => {
            return Either::A(err(error::ErrorBadRequest(
                "Form data key or content is empty",
            )))
        }
    };

    let origin_filename = match params.get_filename() {
        Some(name) => name,
        None => return Either::A(err(error::ErrorBadRequest("No filename found"))),
    };

    let mut vec: Vec<&str> = origin_filename.rsplitn(2, '.').collect();

    let file_name = match vec.pop() {
        Some(name) => name,
        None => return Either::A(err(error::ErrorBadRequest("No file name found"))),
    };

    let file_type = match vec.pop() {
        Some(typ) => {
            if typ != "jpg" && typ != "png" && typ != "gif" {
                return Either::A(err(error::ErrorBadRequest(format!(
                    ".{} can't be uploaded",
                    typ
                ))));
            }
            typ
        }
        None => return Either::A(err(error::ErrorBadRequest("No file extension found"))),
    };

    let mut rng = rand::thread_rng();
    let random_number: u32 = rng.gen();

    let new_filename = format!("{}_{}.{}", &file_name, &random_number, &file_type);

    let file = match fs::File::create(format!("{}{}", "./public/", &new_filename)) {
        Ok(file) => file,
        Err(e) => return Either::A(err(error::ErrorInternalServerError(e))),
    };

    Either::B(
        field
            .fold(
                (UploadResponse::new(origin_filename, new_filename), file),
                move |(response, mut file), bytes| {
                    web::block(move || {
                        file.write_all(bytes.as_ref())
                            .map_err(|e| MultipartError::Payload(error::PayloadError::Io(e)))?;
                        Ok((response, file))
                    })
                    .map_err(
                        |e: error::BlockingError<MultipartError>| match e {
                            error::BlockingError::Error(e) => e,
                            error::BlockingError::Canceled => MultipartError::Incomplete,
                        },
                    )
                },
            )
            .map(|(response, _)| response)
            .map_err(error::ErrorInternalServerError),
    )
}
