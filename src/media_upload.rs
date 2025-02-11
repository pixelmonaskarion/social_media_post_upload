use std::time::Duration;

use aws_config::{load_defaults, BehaviorVersion};
use aws_sdk_s3::{presigning::PresigningConfig, Client};
use lambda_http::{Body, Error, Request, Response};
use uuid::Uuid;

pub async fn media_upload(event: Request) -> Result<Response<Body>, Error> {
    let bytes: Vec<u8> = event.into_body().to_vec();
    let content_id = Uuid::new_v4().to_string();
    
    // Create an S3 client
    let config = load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    
    // Upload the file to S3
    let _result = client
        .put_object()
        .bucket("social-media-post-media")
        .key(&content_id)
        .body(bytes.into())
        .send()
        .await?;

    // Return success response
    let resp = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(format!("{{\"content_id\": \"{}\"}}", content_id).into())
        .map_err(Box::new)?;
    
    Ok(resp)
}

pub async fn media_upload_url(_event: Request) -> Result<Response<Body>, Error> {
    let content_id = Uuid::new_v4().to_string();
    
    // Create an S3 client
    let config = load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    
    // Upload the file to S3
    let presigned_request = client
        .put_object()
        .bucket("social-media-post-media")
        .key(&content_id)
        .presigned(PresigningConfig::expires_in(Duration::from_secs(60*15))?)
        .await?;

    // Return success response
    let resp = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(format!("{{\"content_id\": \"{content_id}\", \"url\": \"{}\"}}", presigned_request.uri()).into())
        .map_err(Box::new)?;
    
    Ok(resp)
}