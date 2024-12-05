use axum::{extract::State, Json};

use std::sync::Arc;

use crate::{db::DatabaseTrait, sensors::interface::SensorController, time::TimeProvider, watering::ds::AppState};

pub async fn list_devices<C: SensorController, D: DatabaseTrait, T: TimeProvider>(
    State(_app_state): State<Arc<AppState<C, D, T>>>,
) -> Json<Vec<String>> {
    // Fetch devices from DB or MQTT
    Json(vec!["Device1".to_string(), "Device2".to_string()])
}

pub async fn query_weather<C: SensorController, D: DatabaseTrait, T: TimeProvider>(
    State(_app_state): State<Arc<AppState<C, D, T>>>,
) -> Json<serde_json::Value> {
    // Fetch recent weather data from DB
    // let weather_data = sqlx::query!("SELECT data FROM weather ORDER BY id DESC LIMIT 1")
    // .fetch_optional(db_pool.as_ref())
    // .await
    // .unwrap()
    // .map(|row| serde_json::from_str(&row.data).unwrap())
    // .unwrap_or(serde_json::json!({}));
    let weather_data = serde_json::from_str("").unwrap();

    Json(weather_data)
}
