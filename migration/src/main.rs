use futures_util::TryStreamExt;
use image::io::Reader as ImageReader;
use image::DynamicImage;
use log::{error, info};
use rusoto_core::{ByteStream, Region};
use rusoto_s3::{GetObjectRequest, PutObjectRequest, S3Client, S3};
use simple_logger::SimpleLogger;

const SIZES: [u32; 1] = [500];

#[tokio::main]
async fn main() {
    SimpleLogger::new().with_utc_timestamps().init().unwrap();
    let s3_client = S3Client::new(Region::default());

    let bucket_name = std::env::var("S3_BUCKET").expect("`S3_BUCKET` env variable not set");
    
    let mut continuation_token: Option<String> = None;

    loop {
        let maybe_list_objects = s3_client
            .list_objects_v2(rusoto_s3::ListObjectsV2Request {
                bucket: bucket_name.clone(),          
                continuation_token: continuation_token.clone(),      
                ..Default::default()
            })
            .await;

        if let Err(e) = maybe_list_objects {
            error!("Error listing objects: {}", e);
            return;
        }

        let list_objects = maybe_list_objects.unwrap();
        handle_s3_objects(&list_objects, &s3_client, &bucket_name).await;        

        if list_objects.next_continuation_token.is_none() {
            break;
        }

        continuation_token = list_objects.next_continuation_token;
    }


    info!("Done listing objects");
}

async fn handle_s3_objects(
    list_objects: &rusoto_s3::ListObjectsV2Output,
    client: &S3Client,
    bucket: &String,
) {
    for object in list_objects.contents.as_ref().unwrap() {
        if let Some(key) = object.key.as_ref() {
            // check that the key is not already a thumbnail
            if key.contains("thumb") {
                // info!("Skipping {} because it is already a thumbnail", &key);
                continue;
            }

            if key.contains("_rrs_w") {
                info!("Skipping {} because it is already resized", &key);
                continue;
            }

            let get_req = GetObjectRequest {
                bucket: bucket.clone(),
                key: key.to_string(),
                ..Default::default()
            };

            let maybe_get_object = client.get_object(get_req).await;

            if let Err(e) = maybe_get_object {
                error!("Error getting object: {}", e);
                continue;
            }

            let get_object = maybe_get_object.unwrap();
            let stream = get_object.body.unwrap();
            let bytes = stream.map_ok(|b| b.to_vec()).try_concat().await;

            if let Err(e) = bytes {
                error!("Error reading object: {}", e);
                continue;
            }

            if let None = get_object.content_type {
                error!("No content type for object: {}", key);
                continue;
            }


            let content_type = get_object.content_type.unwrap();
                                    
            let bytes = bytes.unwrap();
            if let Ok(image) = ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format() {
                if let Ok(dynamic_image) = image.decode() {
                    for resize_width in SIZES.iter() {
                        let new_key = calculate_thumb_name(key, resize_width);

                        // peek rest of list_objects to see if thumbnail already exists
                        if list_objects
                            .contents
                            .as_ref()
                            .unwrap()
                            .iter()
                            .any(|o| o.key.as_ref().unwrap() == &new_key)
                        {
                            info!("Skipping {} size {} because it already exists", &key, &resize_width);
                            continue;
                        }
            

                        let resized_image = resize_image(&dynamic_image, resize_width);
                        let output_format = get_output_format(&content_type);

                        let mut buffer = Vec::new();
                        let resize_message = resized_image
                            .write_to(&mut std::io::Cursor::new(&mut buffer), output_format);

                        if let Err(e) = resize_message {
                            error!("Error resizing image: {}", e);
                            continue;
                        }

                        let put_req = PutObjectRequest {
                            bucket: bucket.to_string(),
                            key: new_key.clone(),
                            body: Some(ByteStream::from(buffer)),
                            content_type: Some(content_type.to_string()),
                            ..Default::default()
                        };

                        if let Err(e) = &client.put_object(put_req).await {
                            error!("Failed to upload thumbnail: {}", e);
                        } else {
                            info!("Uploading thumbnail from {} to {}", &key, &new_key);
                        }
                    }
                }
            }
        }
    }
}

fn get_output_format(content_type: &String) -> image::ImageOutputFormat {
    match content_type.as_str() {
        "image/jpeg" => image::ImageOutputFormat::Jpeg(80),
        "image/png" => image::ImageOutputFormat::Png,
        _ => image::ImageOutputFormat::Unsupported(
            "format is not recognized".to_string(),
        ),
    }
}

fn calculate_thumb_name(key: &String, size: &u32) -> String {
    let file_extension = key.split('.').last().unwrap_or("jpg");
    let new_key = match key.rsplitn(2, '.').collect::<Vec<_>>().last() {
        Some(part) => *part,
        None => key.as_str(),
    };
    let new_key = format!("{}_rrs_w{}.{}", new_key, size, file_extension);
    new_key
}

fn resize_image(img: &DynamicImage, width: &u32) -> DynamicImage {
    let height = img.height() * width / img.width();
    img.resize_exact(width.clone(), height, image::imageops::FilterType::Nearest)
}
