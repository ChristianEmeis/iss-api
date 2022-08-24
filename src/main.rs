use std::time::Duration;

use axum::error_handling::HandleErrorLayer;
use axum::{Router, routing::get};
use reqwest::*;
use serde::{Serialize, Deserialize};
use axum::response::Json;
use serde_json::{Value, json};
use tower::ServiceBuilder;
use tower::limit::RateLimitLayer;
use tower::buffer::BufferLayer;


#[tokio::main]
async fn main() {
    let app = Router::new()
    .route("/trending", get(get_iss_pos))
    .layer(ServiceBuilder::new()
    .layer(HandleErrorLayer::new(|_err| async {
        (StatusCode::REQUEST_TIMEOUT, "timeout")
    }))
    .layer(BufferLayer::new(1024))
    .layer(RateLimitLayer::new(1, Duration::from_secs(2))),);
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
    .serve(app.into_make_service())
    .await
    .unwrap();
}

#[derive(Deserialize, Serialize, Debug)]
pub struct IssPos {
    lat: f64,
    lon: f64,
    height: f64,
    timestamp: i64,
}

pub async fn get_iss_pos() -> (StatusCode, Json<Value>) {
    let mut test1234 = test1.lock().await;
        if test1234.last_updated<chrono::offset::Utc::now().timestamp() - 3600{
            println!("Updated ISS PATH");
            let updated_tle = get_iss_tle().await;
            test1234.line1 = updated_tle.line1;
            test1234.line2 = updated_tle.line2;
            test1234.last_updated = chrono::offset::Utc::now().timestamp();
        }
        let satrec = satellite::io::twoline2satrec(&test1234.line1.as_ref().unwrap(), &test1234.line2.as_ref().unwrap());
        drop(test1234);

    let time = chrono::Utc::now();
    let timestamp = time.timestamp();
    let result = satellite::propogation::propogate_datetime(&mut satrec.as_ref().unwrap(), time).unwrap();

    let gmst = satellite::propogation::gstime::gstime_datetime(time);
    let sat_pos = satellite::transforms::eci_to_geodedic(&result.position, gmst);

    let res = IssPos {
        lat: sat_pos.latitude * satellite::constants::RAD_TO_DEG,
        lon: sat_pos.longitude * satellite::constants::RAD_TO_DEG,
        timestamp: timestamp,
        height: sat_pos.height,
    };
    (StatusCode::OK, Json(json!(res)))  
}

pub struct IssTLE{
    last_updated: i64,
    line1: Option<String>, 
    line2: Option<String>,
}

static test1: async_mutex::Mutex<IssTLE> = async_mutex::Mutex::new(IssTLE {line1: None, line2: None, last_updated: 0});


async fn get_iss_tle() -> IssTLE{
    let url = "https://tle.ivanstanojevic.me/api/tle/25544";
    let client = reqwest::Client::new();
    let response = client.get(url).send().await.unwrap();
    println!("{}", response.status());

    let test: Events;

    match response.json::<Events>().await {
        Ok(parsed) => {
            return IssTLE {line1: Some(parsed.line1.unwrap()), line2: Some(parsed.line2.unwrap()), last_updated: chrono::offset::Utc::now().timestamp()}
        }
        Err(_) => {
            println!("Hm, the response didn't match the shape we expected.");
            return IssTLE {line1: Some("1 25544C 98067A   22200.25763889 -.00062278  00000-0 -10890-2 0   600".to_string()), line2: Some("2 25544  51.6399 177.7528 0005075  27.4260 127.4524 15.49998601    18".to_string()), last_updated: chrono::offset::Utc::now().timestamp()};
        }
    };
}

extern crate serde_derive;

#[derive(Debug, Serialize, Deserialize)]
pub struct Events {
    #[serde(rename = "@context")]
    pub context: Option<String>,
    #[serde(rename = "@id")]
    pub id: Option<String>,
    #[serde(rename = "@type")]
    pub events_type: Option<String>,
    #[serde(rename = "satelliteId")]
    pub satellite_id: Option<i64>,
    pub name: Option<String>,
    pub date: Option<String>,
    pub line1: Option<String>,
    pub line2: Option<String>,
}
