use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),
    #[error("Sensor error: {0}")]
    SensorError(String),
    #[error("Watering error: {0}")]
    WateringError(String),
    #[error("MQTT error: {0}")]
    MQTTError(String),
    #[error("Unknown error")]
    Unknown,
}