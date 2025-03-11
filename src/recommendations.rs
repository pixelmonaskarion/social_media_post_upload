use aws_sdk_dynamodb::types::AttributeValue;
use lambda_http::{Body, Error, Request, RequestExt, Response};

use crate::{info_upload::{get_region_i64, DynamoDBClient}, post_sorting::{sort_posts_by_distance, sort_posts_by_weight, Post}};

pub async fn recommend_posts(event: Request) -> Result<Response<Body>, Error> {
    let params = event.query_string_parameters();
    let Some(location) = params.first("location") else {
        return Ok(Response::builder()
            .status(400)
            .body(Body::from("400 - No location"))
            .unwrap());
    };
    let sorting = params.first("sort_by").unwrap_or("weight");
    let Some((region_long, region_lat)) = get_region_i64(location) else {
        return Ok(Response::builder()
            .status(400)
            .body(Body::from("400 - Invalid location"))
            .unwrap());
    };
    let longitude: f64 = location.split(",").nth(0).unwrap().parse().unwrap();
    let latitude: f64 = location.split(",").nth(1).unwrap().parse().unwrap();
    let client = DynamoDBClient::new().await?;
    let posts = client.client.scan()
        .table_name("SocialMediaPosts")
        .filter_expression("
        (#rlong = :long AND #rlat = :lat) OR
        (#rlong = :longP AND #rlat = :lat) OR
        (#rlong = :long AND #rlat = :latP) OR
        (#rlong = :longP AND #rlat = :latP) OR
        (#rlong = :longN AND #rlat = :lat) OR
        (#rlong = :long AND #rlat = :latN) OR
        (#rlong = :longN AND #rlat = :latN) OR
        (#rlong = :longP AND #rlat = :latN) OR
        (#rlong = :longN AND #rlat = :latP)
        ")
        .expression_attribute_names("#rlong", "r_long")
        .expression_attribute_names("#rlat", "r_lat")
        .expression_attribute_values(":long", AttributeValue::S(format!("{region_long}")))
        .expression_attribute_values(":longP", AttributeValue::S(format!("{}", region_long+1)))
        .expression_attribute_values(":longN", AttributeValue::S(format!("{}", region_long-1)))
        .expression_attribute_values(":lat", AttributeValue::S(format!("{region_lat}")))
        .expression_attribute_values(":latP", AttributeValue::S(format!("{}", region_lat+1)))
        .expression_attribute_values(":latN", AttributeValue::S(format!("{}", region_lat-1)))
        .send().await?.items.unwrap_or_default();
    let mut posts: Vec<Post> = posts.into_iter().filter_map(|it| {
        Post::from_db(it)
    }).collect();
    if sorting == "location" {
        sort_posts_by_distance(&mut posts, longitude, latitude);
    } else {
        sort_posts_by_weight(&mut posts, longitude, latitude);
    }

    let infos = posts.into_iter().map(|item|{ 
        item.content_id
    }).collect::<Vec<_>>();
    
    return Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(format!("[{}]", infos.iter().map(|it| format!("{it:?}")).collect::<Vec<_>>().join(", "))))
        .unwrap());
}