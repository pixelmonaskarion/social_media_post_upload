use std::time::Duration;

use aws_config::{load_defaults, BehaviorVersion};
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_s3::{operation::get_object::GetObjectError, presigning::PresigningConfig};
use lambda_http::{Body, Error, Request, RequestExt, Response};

use crate::info_upload::DynamoDBClient;

pub async fn get_info(event: Request) -> Result<Response<Body>, Error> {
    let params = event.query_string_parameters();
    let Some(content_id) = params.first("content_id") else {
        return Ok(Response::builder()
            .status(400)
            .body(Body::from("400 - No content id"))
            .unwrap());
    };
    let client = DynamoDBClient::new().await?;
    let Ok(Some(item)) = client.get_item("SocialMediaPosts", [("id".into(), AttributeValue::S(content_id.into()))].into()).await else {
        return Ok(Response::builder()
            .status(404)
            .body(Body::from("404 - Post not found"))
            .unwrap());
    };
    let info = item.get("info").unwrap().as_s().unwrap().clone();

    return Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(info))
        .unwrap());
}

pub async fn get_media(event: Request) -> Result<Response<Body>, Error> {
    let params = event.query_string_parameters();
    let Some(content_id) = params.first("content_id") else {
        return Ok(Response::builder()
            .status(400)
            .body(Body::from("400 - No content id"))
            .unwrap());
    };

    let config = load_defaults(BehaviorVersion::latest()).await;
    let client = aws_sdk_s3::Client::new(&config);
    let object = match client.get_object()
        .bucket("social-media-post-media")
        .key(content_id)
        .send()
    .await {
        Ok(object) => object,
        Err(e) => {
            match e.as_service_error() {
                Some(GetObjectError::NoSuchKey(_)) => {
                    return Ok(Response::builder()
                        .status(400)
                        .body(Body::from("404 - Post not found"))
                        .unwrap());
                }
                _ => {
                    return Err(Box::new(e));
                }
            }
        }
    };
    let content_bytes = object.body.collect().await?.into_bytes();

    return Ok(Response::builder()
        .status(200)
        .body(Body::from(content_bytes.as_ref()))
        .unwrap());
}

pub async fn get_media_url(event: Request) -> Result<Response<Body>, Error> {
    let params = event.query_string_parameters();
    let Some(content_id) = params.first("content_id") else {
        return Ok(Response::builder()
            .status(400)
            .body(Body::from("400 - No content id"))
            .unwrap());
    };

    let config = load_defaults(BehaviorVersion::latest()).await;
    let client = aws_sdk_s3::Client::new(&config);
    let presigned_request = match client.get_object()
        .bucket("social-media-post-media")
        .key(content_id)
        .presigned(PresigningConfig::expires_in(Duration::from_secs(60*15))?)
    .await {
        Ok(object) => object,
        Err(e) => {
            match e.as_service_error() {
                Some(GetObjectError::NoSuchKey(_)) => {
                    return Ok(Response::builder()
                        .status(400)
                        .body(Body::from("404 - Post not found"))
                        .unwrap());
                }
                _ => {
                    return Err(Box::new(e));
                }
            }
        }
    };

    return Ok(Response::builder()
        .status(200)
        .body(Body::from(presigned_request.uri()))
        .unwrap());
}