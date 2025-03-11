use std::{collections::HashMap, time::SystemTime};

use aws_config::{load_defaults, BehaviorVersion};
use aws_sdk_dynamodb::{types::AttributeValue, Client};
use lambda_http::{Body, Error, Request, Response};

#[derive(serde::Deserialize)]
struct PostInfo {
    content_id: String,
    location: String,
    username: String,
}

pub async fn info_upload(event: Request) -> Result<Response<Body>, Error> {
    let username_header = event.headers().get("X-Username").unwrap().to_str().unwrap().to_string();
    let Ok(info_string) = String::from_utf8(event.into_body().to_vec()) else {
        return Ok(Response::builder()
            .status(400)
            .body(Body::from("Invalid body"))
            .unwrap());
    };
    let Ok(info) = serde_json::from_str::<PostInfo>(&info_string) else {
        return Ok(Response::builder()
            .status(400)
            .body(Body::from("Invalid body"))
            .unwrap());
    };
    if info.username != username_header {
        return Ok(Response::builder()
            .status(401)
            .body(Body::from("401 - Unauthorized"))
            .unwrap());
    }
    let config = load_defaults(BehaviorVersion::latest()).await;
    let client = aws_sdk_s3::Client::new(&config);

    if !check_file_exists(&client, "social-media-post-media", &info.content_id).await {
        return Ok(Response::builder()
            .status(404)
            .body(Body::from("Content not found"))
            .unwrap());
    
    }

    let Some((r_long, r_lat)) = get_region_i64(&info.location) else {
        return Ok(Response::builder()
            .status(400)
            .body(Body::from("Invalid location"))
            .unwrap());
    };
    let now = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();

    let client = DynamoDBClient::new().await?;
    let mut item = HashMap::new();
    item.insert("id".into(), AttributeValue::S(info.content_id));
    item.insert("r_long".into(), AttributeValue::S(r_long.to_string()));
    item.insert("r_lat".into(), AttributeValue::S(r_lat.to_string()));
    item.insert("region".into(), AttributeValue::S(format!("{r_long},{r_lat}")));
    item.insert("location".into(), AttributeValue::S(info.location));
    item.insert("info".into(), AttributeValue::S(info_string));
    item.insert("date".into(), AttributeValue::N(now.as_millis().to_string()));
    if let Err(_e) = client.put_item("SocialMediaPosts", item).await {
        return Ok(Response::builder()
            .status(500)
            .body(Body::from("Mb :("))
            .unwrap());
    }

    Ok(Response::builder()
            .status(200)
            .body(Body::from(()))
            .unwrap())
}

pub fn get_region(location: &str) -> Option<String> {
    let Some((long, lat)) = get_region_i64(location) else { return None; };
    Some(format!("{long},{lat}"))
}

pub fn get_region_i64(location: &str) -> Option<(i64, i64)> {
    let Some((longitude_str, latitude_str)) = location.split_once(',') else {
        return None;
    };
    let Ok(longitude) = longitude_str.parse::<f64>() else {
        return None;
    };
    let Ok(latitude) = latitude_str.parse::<f64>() else {
        return None;
    };
    Some((longitude.round() as i64, latitude.round() as i64))
}

async fn check_file_exists(client: &aws_sdk_s3::Client, bucket: &str, key: &str) -> bool {
    println!("checking {bucket} / {key}");
    match client
        .head_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
    {
        Ok(_) => true,
        Err(e) => {
            println!("{e:?}");
            return false;
        },
    }
}


pub struct DynamoDBClient {
    pub client: Client,
}

impl DynamoDBClient {
    pub async fn new() -> Result<Self, Error> {
        let config = load_defaults(BehaviorVersion::latest()).await;
        let client = Client::new(&config);
        Ok(Self { client })
    }

    pub async fn put_item(&self, table_name: &str, item: HashMap<String, AttributeValue>) -> Result<(), Error> {
        self.client
            .put_item()
            .table_name(table_name)
            .set_item(Some(item))
            .send()
            .await?;
        Ok(())
    }

    pub async fn get_item(
        &self,
        table_name: &str,
        key: HashMap<String, AttributeValue>,
    ) -> Result<Option<HashMap<String, AttributeValue>>, Error> {
        let response = self.client
            .get_item()
            .table_name(table_name)
            .set_key(Some(key))
            .send()
            .await?;
        
        Ok(response.item)
    }
}