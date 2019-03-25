use std::fs;
use std::io::Write;

use actix_web::{dev, FutureResponse, HttpResponse, HttpRequest, multipart, Error, error, HttpMessage};
use futures::{Future, future, Stream};

use crate::app::AppState;
use crate::handler::auth::UserJwt;




pub fn save_file(
    field: multipart::Field<dev::Payload>, file_path: &str
) -> Box<Future<Item=i64, Error=Error>> {
    let mut file = match fs::File::create(format!("{}{}","./public/",file_path)) {
        Ok(file) => file,
        Err(e) => return Box::new(future::err(error::ErrorInternalServerError(e))),
    };
    Box::new(
        field
            .fold(0i64, move |acc, bytes| {
                let rt = file
                    .write_all(bytes.as_ref())
                    .map(|_| acc + bytes.len() as i64)
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
) -> Box<Stream<Item=i64, Error=Error>> {
    match item {
        multipart::MultipartItem::Field(field) => {
            let params = field.content_disposition().unwrap();
            let filename = params.parameters[1].as_filename().unwrap();
            Box::new(save_file(field, &filename).into_stream())
        }
        multipart::MultipartItem::Nested(mp) => Box::new(
            mp.map_err(error::ErrorInternalServerError)
                .map(move|item| { handle_multipart_item(item, user_id) })
                .flatten(),
        ),
    }
}

pub fn upload_file((user_jwt, req): (UserJwt, HttpRequest<AppState>)) -> FutureResponse<HttpResponse> {
    Box::new(
        req.multipart()
            .map_err(error::ErrorInternalServerError)
            .map(move|item| {
                handle_multipart_item(item, user_jwt.user_id)
            })
            .flatten()
            .collect()
            .map(|sizes| HttpResponse::Ok().json(sizes))
            .map_err(|e| {
                e
            }),
    )
}