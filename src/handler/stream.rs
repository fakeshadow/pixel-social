use actix_multipart::Field;
use bytes::Bytes;
use futures::{compat::Stream01CompatExt, TryStreamExt};
use rand::Rng;
use tokio::{fs::File, io::AsyncWriteExt};

use crate::model::errors::ResError;

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

pub async fn save_file(field: Field) -> Result<UploadResponse, ResError> {
    // need to add an file size limiter here;

    let params = field.content_disposition().ok_or(ResError::BadRequest)?;
    let origin_filename = params.get_filename().ok_or(ResError::BadRequest)?;

    let mut vec: Vec<&str> = origin_filename.rsplitn(2, '.').collect();

    let origin_filename = vec.pop().ok_or(ResError::BadRequest)?;

    let file_type = vec
        .pop()
        .map(|typ| {
            if typ != "jpg" && typ != "png" && typ != "gif" {
                return Err(ResError::BadRequest);
            }
            Ok(typ)
        })
        .ok_or(ResError::BadRequest)??;

    let mut rng = rand::thread_rng();
    let random_number: u32 = rng.gen();

    let new_filename = format!("{}_{}.{}", origin_filename, &random_number, file_type);
    let path = format!("{}{}", "./public/", new_filename.as_str());

    let mut file = File::create(path.as_str())
        .await
        .map_err(|_| ResError::InternalServerError)?;

    let bytes = field
        .compat()
        .try_collect::<Vec<Bytes>>()
        .await
        .map_err(|_| ResError::InternalServerError)?;

    // ToDo: currently only one file can be handled at one time. Need to find a way to handle multiple and nested file upload.

    let bytes = bytes.first().ok_or(ResError::InternalServerError)?;

    file.write_all(bytes.as_ref())
        .await
        .map_err(|_| ResError::InternalServerError)?;

    Ok(UploadResponse::new(origin_filename, new_filename))
}
