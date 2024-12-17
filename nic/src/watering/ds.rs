use super::modes::Mode;
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

pub type WeeklyPlan = Vec<(i64, DailyPlan)>; // A week's plan: date -> daily plan

#[derive(Debug, Default, Clone, PartialEq)]
pub struct DailyPlan(pub Vec<WaterSector>); // A day's plan: (sector_id , start time,  duration)

impl DailyPlan {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn is_watering_time(&self, current_time: i64) -> bool {
        self.0.first().map_or(false, |first_sector| first_sector.start <= current_time)
    }

    pub fn get_cycle(&self, current_time: i64) -> Option<Cycle> {
        self.is_watering_time(current_time).then(|| Cycle::build(self.clone()))
    }
}

#[derive(Debug, Clone, Default)]
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
    /// last watered
    pub last_water: i64,
}

impl SectorInfo {
    pub fn build(
        id: u32, weekly_target: f64, sprinkler_debit: f64, max_duration: i64, progress: f64, percolation_rate: f64,
        last_water: i64,
    ) -> SectorInfo {
        SectorInfo { id, weekly_target, sprinkler_debit, percolation_rate, max_duration, progress, last_water }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Ord, PartialOrd, Eq)]
pub struct WaterSector {
    pub id: u32,
    pub start: i64,
    /// in seconds
    pub duration: i64,
}

impl WaterSector {
    pub fn new(id: u32, start: i64, duration: i64) -> Self {
        Self { id, start, duration }
    }

    pub fn duration_minutes(&self)->f64{
        self.duration as f64 / 60.
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Cycle {
    pub id: i64,
    pub daily_plan: DailyPlan,
    pub curr_sector: usize,
}

impl Cycle {
    pub fn build(daily_plan: DailyPlan) -> Self {
        assert!(!daily_plan.0.is_empty());
        Cycle { id: daily_plan.0[0].start, daily_plan, curr_sector: usize::MAX }
    }

    pub fn get_start(&self) -> Option<i64> {
        self.daily_plan.0.first().map(|sector| sector.start)
    }

    pub fn get_start_unchecked(&self) -> i64 {
        self.daily_plan.0[0].start
    }

    pub fn next_sector(&mut self) -> Option<WaterSector> {
        self.curr_sector = self.curr_sector.wrapping_add(1);
        self.daily_plan.0.get(self.curr_sector).copied()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WeatherSignal {
    RainStart,
    RainStop,
    HighWind,
    LowWind,
}

#[derive(Debug, Clone)]
pub enum CtrlSignal {
    Weather(WeatherSignal),
    StopMachine,
    GenWeather(String),
    DevicesState(String),
    ChgMode(Mode),
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

pub struct AppState {
    pub db: Arc<dyn DatabaseTrait>,
    pub sm_tx: Arc<Sender<CtrlSignal>>,
    pub web_rx: Arc<Mutex<Receiver<CtrlSignal>>>,
    pub web_tx: Arc<Sender<CtrlSignal>>,
    pub sm_rx: Arc<Mutex<Receiver<CtrlSignal>>>,
    pub sensors_ctrl: Arc<dyn SensorController>,
    pub time_provider: Arc<dyn TimeProvider>,
}

impl AppState {
    pub async fn new(
        db: Arc<dyn DatabaseTrait>, sensors_ctrl: Arc<dyn SensorController>, time_provider: Arc<dyn TimeProvider>,
        sm_tx: Arc<Sender<CtrlSignal>>, sm_rx: Arc<Mutex<Receiver<CtrlSignal>>>, web_tx: Arc<Sender<CtrlSignal>>,
        web_rx: Arc<Mutex<Receiver<CtrlSignal>>>,
    ) -> Result<Arc<Self>, AppError> {
        Ok(Arc::new(AppState { db, sm_tx, sm_rx, web_tx, web_rx, sensors_ctrl, time_provider }))
    }
}

#[derive(Debug)]
pub struct WateringEvent {
    pub cycle_id: Option<u32>,
    pub sector: WaterSector,
    pub water_applied: f64,
    pub mode: Mode,
}

impl WateringEvent {
    pub fn new(cycle_id: Option<u32>, sector: WaterSector, water_applied: f64, mode: Mode) -> Self {
        Self { cycle_id, sector, water_applied, mode }
    }
}
