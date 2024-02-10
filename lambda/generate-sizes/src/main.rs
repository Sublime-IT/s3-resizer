use std::ops::ControlFlow;

use futures_util::stream::TryStreamExt;
use image::io::Reader as ImageReader;
use image::DynamicImage;
use lambda_runtime::{handler_fn, Context, Error};
use log::{error, info, warn};
use rusoto_core::ByteStream;
use rusoto_core::Region;
use rusoto_s3::{GetObjectRequest, HeadObjectError, PutObjectRequest, S3Client, S3};
use simple_logger::SimpleLogger;

const SIZES: [u32; 10] = [
    128, 256, 384, 640, 750, 828, 1080, 1200, 1440, 1920
];
const CONVERT_TO_WEBP: bool = true;
const SKIP_UPSCALING: bool = true;

#[tokio::main]
async fn main() -> Result<(), Error> {
    SimpleLogger::new().with_utc_timestamps().init().unwrap();

    let func = handler_fn(func);
    lambda_runtime::run(func).await?;
    Ok(())
}

async fn func(event: serde_json::Value, _: Context) -> Result<(), Error> {
    let s3_client = S3Client::new(Region::default());

    if let Some(records) = event["Records"].as_array() {
        for record in records {
            let bucket = record["s3"]["bucket"]["name"].as_str().unwrap();
            let key = record["s3"]["object"]["key"]
                .as_str()
                .unwrap()
                .replace("+", " ");

            info!("Processing file: {}/{}", bucket, key);

            // verify that the object is an image (content type is image/*)
            let head_req = rusoto_s3::HeadObjectRequest {
                bucket: bucket.to_string(),
                key: key.to_string(),
                ..Default::default()
            };

            // s3_client.head_object(head_req).await;
            let head_result = s3_client.head_object(head_req).await;
            if let Err(e) = head_result {
                error!("Failed to get object metadata: {}", e);

                // Check file not found.
                if let rusoto_core::RusotoError::Service(HeadObjectError::NoSuchKey(_)) = e {
                    warn!("File not found: {}/{}", bucket, key);
                    continue;
                }

                // Check if it's an permission error
                if let rusoto_core::RusotoError::Unknown(response) = e {
                    if response.status == 403 {
                        error!("Permission denied: {}/{}", bucket, key);
                        continue;
                    } else {
                        error!(
                            "Unknown error: {}/{}. Return status: {}",
                            bucket, key, response.status
                        );
                        continue;
                    }
                }

                continue;
            }

            let mut maybe_content_type: Option<String> = None;
            if let Some(ct) = head_result?.content_type {
                if !ct.starts_with("image/") {
                    warn!(
                        "File is not an image: {}/{}. It had content-type {}",
                        bucket, key, ct
                    );
                    continue;
                }
                maybe_content_type = Some(ct);
            }

            if let None = &maybe_content_type {
                error!("No content type found for file: {}/{}", bucket, key);
                continue;
            }
            let content_type = maybe_content_type
                .as_ref()
                .expect("Content type should be set at this point");

            // verify that the object is not a thumbnail
            if key.contains("_rrs_w") {
                info!(
                    "Skipping file because it is already a resize file: {}/{}",
                    bucket, key
                );
                continue;
            }

            // resize the image and upload it to same place with a new key
            let get_req = GetObjectRequest {
                bucket: bucket.to_string(),
                key: key.to_string(),
                ..Default::default()
            };

            let result = s3_client.get_object(get_req).await?;
            let stream = result.body
                .expect("No body found in the response");
            let bytes: Vec<u8> = stream.map_ok(|b| b.to_vec()).try_concat().await?;

            if let Ok(image) = ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format() {
                if let Ok(dynamic_image) = image.decode() {
                    for resize_width in SIZES.iter() {
                        let resized_image = resize_image(&dynamic_image, &resize_width)?;
                        let mut buffer = Vec::new();

                        let content_type = if CONVERT_TO_WEBP {
                            "image/webp".to_string()
                        } else {
                            content_type.clone()
                        };

                        let image_format = match content_type.as_str() {
                            "image/jpeg" => image::ImageOutputFormat::Jpeg(90),
                            "image/png" => image::ImageOutputFormat::Png,
                            "image/webp" => image::ImageOutputFormat::WebP,
                            _ => {
                                error!("Unsupported image format: {}", &content_type);
                                continue;
                            }
                        };

                        let resize_message = resized_image
                            .write_to(&mut std::io::Cursor::new(&mut buffer), image_format);

                        if let ControlFlow::Break(_) = upload_image(
                            resize_message,
                            &content_type,
                            &key,
                            resize_width,
                            bucket,
                            buffer,
                            &s3_client,
                        )
                        .await
                        {
                            continue;
                        }
                    }
                } else {
                    error!("Failed to decode image: {}/{}", bucket, key);
                }
            } else {
                error!("Failed to read image format: {}/{}", bucket, key);
            }
        }
    } else {
        warn!("No records found in the event");
    }

    Ok(())
}

async fn upload_image(
    resize_message: Result<(), image::ImageError>,
    content_type: &String,
    key: &String,
    resize_width: &u32,
    bucket: &str,
    buffer: Vec<u8>,
    s3_client: &S3Client,
) -> ControlFlow<()> {
    if let Err(e) = resize_message {
        error!("Failed to resize image: {}", e);
        return ControlFlow::Break(());
    }
    let file_extension = match content_type.as_str() {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        _ => {
            error!(
                "Unsupported file extension for content type: {}",
                &content_type
            );
            return ControlFlow::Break(());
        }
    };
    let new_key = match key.rsplitn(2, '.').collect::<Vec<_>>().last() {
        Some(part) => *part,
        None => key.as_str(),
    };
    let new_key = format!("{}_rrs_w{}.{}", new_key, &resize_width, file_extension);
    let put_req = PutObjectRequest {
        bucket: bucket.to_string(),
        key: new_key,
        body: Some(ByteStream::from(buffer)),
        content_type: Some(content_type.to_string()),
        ..Default::default()
    };

    if let Err(e) = s3_client.put_object(put_req).await {
        error!("Failed to upload thumbnail: {}", e);
    } else {
        info!("Uploaded thumbnail");
    }
    ControlFlow::Continue(())
}

fn resize_image(img: &DynamicImage, width: &u32) -> Result<DynamicImage, Error> {
    if img.width() <= *width && SKIP_UPSCALING {
        return Ok(img.clone());
    }

    let height = img.height() * width / img.width();
    Ok(img.resize_exact(width.clone(), height, image::imageops::FilterType::Nearest))
}
