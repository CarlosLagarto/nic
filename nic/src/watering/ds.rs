use super::watering_system::WateringSystem;
use crate::{db::DatabaseTrait, error::AppError, sensors::interface::SensorController};
use chrono::Duration;
use std::sync::Arc;

pub type DailyPlan = Vec<(u32, i64, i64)>; // A day's plan: (sector_id , start time,  duration)
pub type WeeklyPlan = Vec<(i64, DailyPlan)>; // A week's plan: date -> daily plan

#[derive(Debug, Clone)]
pub struct SectorInfo {
    pub id: u32,
    /// cm /hour
    pub sprinkler_debit: f64, // cm/hour (sprinkler output rate)
    /// mm/hour
    pub percolation_rate: f64, // mm/hour (soil percolation rate)
    /// in seconds
    pub max_duration: i64, // Maximum safe watering duration per session, in seconds
    /// cm
    pub weekly_target: f64, // Weekly water target (cm)
    /// current progress
    pub progress: f64,
}

impl SectorInfo {
    pub fn build(
        id: u32, weekly_target: f64, sprinkler_debit: f64, max_duration: i64, progress: f64, percolation_rate: f64,
    ) -> SectorInfo {
        SectorInfo { id, weekly_target, sprinkler_debit, percolation_rate, max_duration, progress }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Cycle {
    pub id: u32,
    pub instructions: Vec<(u32, i64)>, // (Sector ID, Duration)
}

#[derive(Debug, Clone, PartialEq)]
pub enum WateringState {
    Idle,               // No active watering
    Activating(u32),    // Activating a sector
    Watering(u32, i64), // Actively watering (sector ID, duration)
    Deactivating(u32),  // Deactivating a sector
}

#[derive(Debug, Clone)]
pub enum EnvironmentalSignal {
    RainStart,
    RainStop,
    HighWind,
    LowWind,
}
#[derive(Debug, Clone)]
pub enum ControlSignal {
    Environmental(EnvironmentalSignal),
    StopMachine,
    Weather(String),
    DevicesState(String),
    SwitchToAuto,
    SwitchToManual,
    SwitchToWizard,
}

pub struct WeatherConditions {
    pub is_raining: bool,
    pub wind_speed: f64, // in km/h or m/s
    pub temperature: f64,
    pub humidity: f64,
    pub solar_radiation: f64,
}

pub struct AppState<C: SensorController, D: DatabaseTrait> {
    pub db: Arc<D>,
    pub watering_system: Arc<WateringSystem<C>>,
}

impl<C: SensorController + 'static, D: DatabaseTrait + 'static> AppState<C, D> {
    pub async fn new(db: Arc<D>, sensors_ctrl: Arc<C>) -> Result<Arc<Self>, AppError> {
        let watering_system = WateringSystem::new(sensors_ctrl, db.clone()).await?;
        Ok(Arc::new(AppState { db, watering_system }))
    }
}

#[derive(Debug)]
pub struct WateringEvent {
    pub cycle_id: Option<u32>,
    pub sector_id: u32,
    pub start_time: String,
    pub duration: Duration,
    pub water_applied: f64,
    pub event_type: EventType,
}

impl WateringEvent {
    pub fn new(
        cycle_id: Option<u32>, sector_id: u32, start_time: String, duration: Duration, water_applied: f64,
        event_type: EventType,
    ) -> Self {
        Self { cycle_id, sector_id, start_time, duration, water_applied, event_type }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    Auto,
    Manual,
    Wizard,
}

impl ToString for EventType {
    fn to_string(&self) -> String {
        match self {
            EventType::Auto => "auto".to_string(),
            EventType::Manual => "manual".to_string(),
            EventType::Wizard => "wizard".to_string(),
        }
    }
}
