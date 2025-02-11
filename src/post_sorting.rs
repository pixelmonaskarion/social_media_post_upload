use std::{cmp::Ordering, collections::HashMap, time::{Duration, SystemTime}};

use aws_sdk_dynamodb::types::AttributeValue;

#[derive(Debug, Clone)]
pub struct Post {
    pub likes: f64,
    pub time_since_created: f64,
    pub lat: f64,
    pub long: f64,
    pub content_id: String,
}

impl Post {
    pub fn distance(&self, current_long: f64, current_lat: f64) -> f64 {
        ((self.long-current_long).powi(2)+(self.lat-current_lat).powi(2)).sqrt()
    }

    pub fn weight(&self, current_long: f64, current_lat: f64) -> f64 {
        self.likes / (self.distance(current_long, current_lat) + self.time_since_created)
    }

    pub fn from_db(map: HashMap<String, AttributeValue>) -> Option<Self> {
        let likes = map.get("likes").map(|it| it.as_n().unwrap().parse().unwrap()).unwrap_or(0.);
        let Some(timestamp) = map.get("date") else { return None; };
        let timestamp: u64 = timestamp.as_n().unwrap().parse().unwrap();
        let time_since_created = SystemTime::UNIX_EPOCH.elapsed().unwrap()-Duration::from_millis(timestamp);
        let time_since_created = time_since_created.as_secs_f32() as f64/60./60.;
        let Some(location) = map.get("location") else { return None; };
        let location = location.as_s().unwrap().clone();
        let long: f64 = location.split(",").nth(0).unwrap().parse().unwrap();
        let lat: f64 = location.split(",").nth(1).unwrap().parse().unwrap();
        let Some(content_id) = map.get("id") else { return None; };
        let content_id = content_id.as_s().unwrap().clone();
        Some(Self {
            likes,
            time_since_created,
            long,
            lat,
            content_id, 
        })
    }
}

pub fn sort_posts_by_weight(posts: &mut [Post], current_long: f64, current_lat: f64) {
    posts.sort_by(|a, b| {
        b.weight(current_long, current_lat).partial_cmp(&a.weight(current_long, current_lat))
            .unwrap_or(Ordering::Equal)
    });
}

pub fn sort_posts_by_distance(posts: &mut [Post], current_long: f64, current_lat: f64) {
    posts.sort_by(|a, b| {
        a.distance(current_long, current_lat).partial_cmp(&b.distance(current_long, current_lat))
            .unwrap_or(Ordering::Equal)
    });
}