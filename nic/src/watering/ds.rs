use super::state_machine::WateringSystem;
use crate::db::Database;
use chrono::Duration;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SectorInfo {
    pub id: u32,
    pub sprinkler_debit: f64,   // cm/hour (sprinkler output rate)
    pub percolation_rate: f64,  // mm/hour (soil percolation rate)
    pub max_duration: Duration, // Maximum safe watering duration per session
    pub weekly_target: f64,     // Weekly water target (cm)
}

#[derive(Debug, Default, Clone)]
pub struct Cycle {
    pub id: u32,
    pub instructions: Vec<(u32, Duration)>, // (Sector ID, Duration)
}

#[derive(Debug, Clone, PartialEq)]
pub enum WateringState {
    Idle,              // No active watering
    Activating(u32),   // Activating a sector
    Watering(u32),     // Currently watering a sector
    Deactivating(u32), // Deactivating a sector
}

#[derive(Debug, Clone)]
pub enum EnvironmentalSignal {
    RainStart,
    RainStop,
    HighWind,
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
}

pub struct AppState {
    pub db: Database,
    pub watering_system: Arc<WateringSystem>,
}

pub struct WateringEvent {
    pub cycle_id: Option<u32>,
    pub sector_id: u32,
    pub start_time: String,
    pub duration: Duration,
    pub water_applied: f64,
    pub event_type: String,
}

impl WateringEvent {
    pub fn new(
        cycle_id: Option<u32>,
        sector_id: u32,
        start_time: String,
        duration: Duration,
        water_applied: f64,
        event_type: String,
    ) -> Self {
        Self {
            cycle_id,
            sector_id,
            start_time,
            duration,
            water_applied,
            event_type,
        }
    }
}