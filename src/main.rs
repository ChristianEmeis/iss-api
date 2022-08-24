use axum::error_handling::HandleErrorLayer;
use axum::response::Json;
use axum::{routing::get, Router};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use reqwest::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tower::buffer::BufferLayer;
use tower::limit::RateLimitLayer;
use tower::ServiceBuilder;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/isspos", get(get_iss_pos))
        .route("/isspath", get(get_iss_path))
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|_err| async {
                    (StatusCode::REQUEST_TIMEOUT, "timeout")
                }))
                .layer(BufferLayer::new(1024))
                .layer(RateLimitLayer::new(5, Duration::from_secs(60))),
        );
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
    let mut iss_tle_mutex = ISSTLE.lock().await;
    if iss_tle_mutex.last_updated < chrono::offset::Utc::now().timestamp() - 3600 {
        println!("Updated ISS PATH");
        let updated_tle = get_iss_tle().await;
        iss_tle_mutex.line1 = updated_tle.line1;
        iss_tle_mutex.line2 = updated_tle.line2;
        iss_tle_mutex.last_updated = chrono::offset::Utc::now().timestamp();
    }
    let satrec = satellite::io::twoline2satrec(
        &iss_tle_mutex.line1.as_ref().unwrap(),
        &iss_tle_mutex.line2.as_ref().unwrap(),
    );
    drop(iss_tle_mutex);

    let time = chrono::Utc::now();
    let timestamp = time.timestamp();
    let result =
        satellite::propogation::propogate_datetime(&mut satrec.as_ref().unwrap(), time).unwrap();

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

pub struct IssTLE {
    last_updated: i64,
    line1: Option<String>,
    line2: Option<String>,
}

static ISSTLE: async_mutex::Mutex<IssTLE> = async_mutex::Mutex::new(IssTLE {
    line1: None,
    line2: None,
    last_updated: 0,
});

async fn get_iss_tle() -> IssTLE {
    let url = "https://tle.ivanstanojevic.me/api/tle/25544";
    let client = reqwest::Client::new();
    let response = client.get(url).send().await.unwrap();

    match response.json::<Events>().await {
        Ok(parsed) => {
            return IssTLE {
                line1: Some(parsed.line1.unwrap()),
                line2: Some(parsed.line2.unwrap()),
                last_updated: chrono::offset::Utc::now().timestamp(),
            }
        }
        Err(_) => {
            println!("Hm, the response didn't match the shape we expected.");
            return IssTLE {
                line1: Some(
                    "1 25544C 98067A   22200.25763889 -.00062278  00000-0 -10890-2 0   600"
                        .to_string(),
                ),
                line2: Some(
                    "2 25544  51.6399 177.7528 0005075  27.4260 127.4524 15.49998601    18"
                        .to_string(),
                ),
                last_updated: chrono::offset::Utc::now().timestamp(),
            };
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Point {
    lat: f64,
    lon: f64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Path {
    time: Option<DateTime<Utc>>,
    path: Vec<Point>,
}

static ISSPATH: async_mutex::Mutex<Path> = async_mutex::Mutex::new(Path {
    time: None,
    path: Vec::new(),
});

async fn get_iss_path() -> (StatusCode, Json<Value>) {
    let mut iss_tle_mutex = ISSTLE.lock().await;
    let mut path_mutex = ISSPATH.lock().await;
    if path_mutex.time.is_none()
        || (path_mutex.time.unwrap().timestamp() <= Utc::now().timestamp() - 30)
    {
        if iss_tle_mutex.last_updated < chrono::offset::Utc::now().timestamp() - 3600 {
            println!("Updated ISS PATH");
            let updated_tle = get_iss_tle().await;
            iss_tle_mutex.line1 = updated_tle.line1;
            iss_tle_mutex.line2 = updated_tle.line2;
            iss_tle_mutex.last_updated = chrono::offset::Utc::now().timestamp();
        }
        let satrec = satellite::io::twoline2satrec(
            &iss_tle_mutex.line1.as_ref().unwrap(),
            &iss_tle_mutex.line2.as_ref().unwrap(),
        );
        drop(iss_tle_mutex);
        let mut time = Utc::now();
        let now = time;

        let end = time + ChronoDuration::minutes(92);

        let mut vec = Vec::new();

        while time <= end {
            let result =
                satellite::propogation::propogate_datetime(&mut satrec.as_ref().unwrap(), time)
                    .unwrap();

            let gmst = satellite::propogation::gstime::gstime_datetime(time);
            let sat_pos = satellite::transforms::eci_to_geodedic(&result.position, gmst);
            let temp_point = Point {
                lat: sat_pos.latitude * satellite::constants::RAD_TO_DEG,
                lon: sat_pos.longitude * satellite::constants::RAD_TO_DEG,
            };
            vec.push(temp_point);
            time = time + ChronoDuration::seconds(60);
        }
        path_mutex.time = Some(now);
        path_mutex.path = vec;
    }
    let return_path = Path {
        time: path_mutex.time,
        path: path_mutex.path.clone(),
    };
    drop(path_mutex);
    (StatusCode::OK, Json(json!(return_path)))
}
