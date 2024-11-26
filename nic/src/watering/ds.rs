use super::{interface::SensorController, state_machine::WateringSystem};
use crate::db::Database;

use chrono::Duration;
use std::sync::Arc;
#[derive(Debug, Clone)]
pub struct SectorInfo {
    pub id: u32,
    /// cm /hour
    pub sprinkler_debit: f64, // cm/hour (sprinkler output rate)
    /// mm/hour
    pub percolation_rate: f64, // mm/hour (soil percolation rate)
    /// in minutes
    pub max_duration: Duration, // Maximum safe watering duration per session
    /// cm
    pub weekly_target: f64, // Weekly water target (cm)
}

#[derive(Debug, Default, Clone, PartialEq)]
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
}

pub struct AppState<C: SensorController> {
    pub db: Database,
    pub watering_system: Arc<WateringSystem<C>>,
}

impl<C: SensorController> AppState<C> {
    pub async fn new(db: Database, controler: Arc<C>) -> Arc<Self> {
        let watering_system = WateringSystem::new(controler).await;
        Arc::new(AppState {
            db,
            watering_system,
        })
    }
}

#[derive(Debug)]
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
