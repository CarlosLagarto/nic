use super::modes::ModeIdx;
use crate::{
    api::{CycleResponse, WateringStateResponse},
    db::DatabaseTrait,
    error::AppError,
    sensors::interface::SensorController,
    time::TimeProvider,
};
use std::sync::Arc;
use tokio::sync::{
    broadcast::{Receiver, Sender},
    Mutex,
};

pub type DailyPlan = Vec<WaterSector>; // A day's plan: (sector_id , start time,  duration)
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Ord, PartialOrd, Eq)]
pub struct WaterSector {
    pub id: u32,
    pub start: i64,
    pub duration: i64,
}

impl WaterSector {
    pub fn new(id: u32, start: i64, duration: i64) -> Self {
        Self { id, start, duration }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Cycle {
    pub id: u32,
    pub instructions: Vec<WaterSector>, // (Sector ID, Duration)
}

#[derive(Debug, Clone, PartialEq)]
pub enum WateringState {
    Idle,                      // No active watering
    Activating(WaterSector),   // Activating a sector (sector id, start time, duration)
    Watering(WaterSector),     // Actively watering (sector ID, start time, duration)
    Deactivating(WaterSector), // Deactivating a sector
}

impl WateringState {
    pub fn get_current_sector_id(&self) -> Option<u32> {
        match self {
            WateringState::Activating(sec) => Some(sec.id),
            WateringState::Watering(sec) => Some(sec.id),
            WateringState::Deactivating(sec) => Some(sec.id),
            _ => None, // Idle or other states that don't involve a specific sector
        }
    }
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
    GetState,
    GetStateResponse(WateringStateResponse),
    GetCycle,
    GetCycleResponse(CycleResponse),
}

pub struct WeatherConditions {
    pub is_raining: bool,
    pub wind_speed: f64, // in km/h or m/s
    pub temperature: f64,
    pub humidity: f64,
    pub solar_radiation: f64,
}

pub struct AppState<C: SensorController, D: DatabaseTrait, T: TimeProvider> {
    pub db: Arc<D>,
    pub tx: Arc<Sender<ControlSignal>>,
    pub rx: Arc<Mutex<Receiver<ControlSignal>>>,
    pub sensors_ctrl: Arc<C>,
    pub time_provider: Arc<T>,
}

impl<C: SensorController + 'static, D: DatabaseTrait + 'static, T: TimeProvider + 'static> AppState<C, D, T> {
    pub async fn new(
        db: Arc<D>, sensors_ctrl: Arc<C>, time_provider: Arc<T>, tx: Arc<Sender<ControlSignal>>,
        rx: Arc<Mutex<Receiver<ControlSignal>>>,
    ) -> Result<Arc<Self>, AppError> {
        Ok(Arc::new(AppState { db, tx, rx, sensors_ctrl, time_provider }))
    }
}

#[derive(Debug)]
pub struct WateringEvent {
    pub cycle_id: Option<u32>,
    pub sector: WaterSector,
    pub water_applied: f64,
    pub mode: ModeIdx,
}

impl WateringEvent {
    pub fn new(cycle_id: Option<u32>, sector: WaterSector, water_applied: f64, mode: ModeIdx) -> Self {
        Self { cycle_id, sector, water_applied, mode }
    }
}
