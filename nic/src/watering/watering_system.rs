use super::{
    ds::{AppState, ControlSignal, Cycle, EventType, SectorInfo, WateringState},
    mode::ModeEnum,
    mode_auto::ModeAuto,
    mode_manual::ModeManual,
    mode_wizard::ModeWizard,
    schedule::{AllowedTimeframe, Schedule},
    state_machine::WateringStateMachine,
};
use crate::{
    db::DatabaseTrait,
    error::AppError,
    sensors::interface::SensorController,
    utils::{display_from_ts, sod},
    watering::ds::WateringEvent,
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{broadcast, Mutex, RwLock};
use tracing::{debug, info};

pub struct WateringSystem<C: SensorController> {
    pub sectors: Arc<RwLock<HashMap<u32, SectorInfo>>>,
    pub timeframe: Arc<RwLock<AllowedTimeframe>>,
    pub state_machine: Arc<RwLock<WateringStateMachine>>,
    pub auto_mode: Arc<RwLock<ModeAuto>>,
    pub manual_mode: Arc<RwLock<ModeManual>>,
    pub wizard_mode: Arc<RwLock<ModeWizard>>,
    pub active_mode: Arc<RwLock<ModeEnum>>, // Tracks the currently active mode
    pub controller: Arc<C>,                 // Sensor controller (mockable)
}

impl<C: SensorController + 'static> WateringSystem<C> {
    pub async fn new<D: DatabaseTrait>(controller: Arc<C>, db: Arc<D>) -> Result<Arc<Self>, AppError> {
        let timeframe = Arc::new(RwLock::new(AllowedTimeframe::new(22, 8)));

        let sectors: Vec<SectorInfo> = db.load_sectors()?;
        let sectors = Arc::new(RwLock::new(load_sectors_into_hashmap(sectors)));

        let state_machine = WateringStateMachine::new();

        let (auto_mode, manual_mode, wizard_mode) = Self::initialize_modes(&db);

        Ok(Arc::new(WateringSystem {
            state_machine: Arc::new(RwLock::new(state_machine)),
            sectors,
            timeframe,
            controller,
            auto_mode: Arc::new(RwLock::new(auto_mode.clone())),
            manual_mode: Arc::new(RwLock::new(manual_mode)),
            wizard_mode: Arc::new(RwLock::new(wizard_mode)),
            active_mode: Arc::new(RwLock::new(ModeEnum::Auto(auto_mode))),
        }))
    }

    pub async fn switch_mode(&self, new_mode: ModeEnum) {
        let mut active_mode = self.active_mode.write().await;

        match &new_mode {
            ModeEnum::Auto(_) => {
                debug!("Switching to Auto Mode");
            }
            ModeEnum::Manual(_) => {
                debug!("Switching to Manual Mode");
            }
            ModeEnum::Wizard(_) => {
                debug!("Switching to Wizard Mode");
            }
        }

        *active_mode = new_mode;
    }

    fn initialize_modes<D: DatabaseTrait>(db: &Arc<D>) -> (ModeAuto, ModeManual, ModeWizard) {
        // Read schedules from the database
        let auto_schedule = db.load_auto_schedule().unwrap_or_else(|_| {
            // Fallback to an empty schedule if the database fails to load
            Schedule::new(vec![])
        });

        // Initialize Auto Mode
        let auto_mode = ModeAuto::new(Cycle::default(), auto_schedule);

        // Initialize Manual Mode
        let manual_mode = ModeManual::new(Cycle::default());

        // initialize Wizard Schedule
        let wizard_schedule = Schedule::new(vec![]);
        let wizard_mode = ModeWizard::new(wizard_schedule);

        (auto_mode, manual_mode, wizard_mode)
    }

    async fn handle_signal(&self, signal: ControlSignal, state_machine: &mut WateringStateMachine) {
        match signal {
            ControlSignal::SwitchToAuto => {
                let auto_mode = self.auto_mode.read().await.clone();
                self.switch_mode(ModeEnum::Auto(auto_mode)).await;
            }
            ControlSignal::SwitchToManual => {
                let manual_mode = self.manual_mode.read().await.clone();
                self.switch_mode(ModeEnum::Manual(manual_mode)).await;
            }
            ControlSignal::SwitchToWizard => {
                let wizard_mode = self.wizard_mode.read().await.clone();
                self.switch_mode(ModeEnum::Wizard(wizard_mode)).await;
            }
            ControlSignal::Environmental(env_signal) => {
                let mut active_mode = self.active_mode.write().await;
                if let ModeEnum::Wizard(wizard_mode) = &mut *active_mode {
                    wizard_mode.handle_signal(env_signal, state_machine);
                }
            }
            ControlSignal::StopMachine => {
                state_machine.state = WateringState::Idle;
                state_machine.cycle = None;
            }
            ControlSignal::Weather(_weather) => {}    //TODO:
            ControlSignal::DevicesState(_state) => {} //TODO:
        }
    }

    async fn handle_idle(&self, sector_id: u32) {
        let mut sm = self.state_machine.write().await;
        sm.state = WateringState::Activating(sector_id);
        self.controller.activate_sector(sector_id).await;
        info!("State transitioned to Activating for sector {}", sector_id);
    }

    pub async fn handle_activating<D: DatabaseTrait + 'static>(
        &self, duration: i64, db: &Arc<D>, event_type: EventType, sector: &SectorInfo,
    ) {
        let mut sm = self.state_machine.write().await;
        sm.state = WateringState::Watering(sector.id, duration);
        info!("State transitioned to Watering for sector {} with duration {:?}", sector.id, duration);

        let controller_clone = self.controller.clone();
        let db_clone = db.clone();
        let sector_clone = sector.clone();
        let sector_id_clone = sector.id;
        let total_duration_secs = duration;
        let secs = self.sectors.clone();
        let mut elapsed_secs = 0;

        tokio::spawn(async move {
            while elapsed_secs < total_duration_secs {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                elapsed_secs += 1;
                let incremental_progress = (1.0 / 3600.0) * sector_clone.sprinkler_debit;
                {
                    let mut secs = secs.write().await;
                    if let Some(sec) = secs.get_mut(&sector_id_clone) {
                        sec.progress += incremental_progress;
                        info!("Sector {} watering progress: {:.2} cm", sector_id_clone, sec.progress);
                    }
                }
                // Handle potential interruptions
                if elapsed_secs >= total_duration_secs {
                    info!("Completed watering for sector {}", sector_id_clone);
                    let water_applied = (elapsed_secs as f64 / 3600.0) * sector_clone.sprinkler_debit; // Final water applied

                    _ = db_clone.log_watering_event(WateringEvent::new(
                        None,
                        sector_id_clone,
                        chrono::Local::now().to_rfc3339(),
                        chrono::Duration::seconds(elapsed_secs as i64),
                        water_applied,
                        event_type,
                    ));

                    controller_clone.deactivate_sector(sector_id_clone).await;
                    break;
                }
            }
        });
    }

    async fn handle_deactivating(&self, sector_id: u32) {
        let mut sm = self.state_machine.write().await;
        sm.state = WateringState::Idle;

        self.controller.deactivate_sector(sector_id).await;
        sm.current_instruction += 1;
        info!("State transitioned to Idle after deactivating sector {}", sector_id);
    }

    pub async fn update<D: DatabaseTrait + 'static>(&self, db: &Arc<D>, event_type: EventType) {
        let sm = self.state_machine.read().await;

        if let Some(cycle) = &sm.cycle {
            if sm.current_instruction >= cycle.instructions.len() {
                let mut sm = self.state_machine.write().await;
                info!("Cycle completed.");
                sm.cycle = None;
                sm.state = WateringState::Idle;
                return;
            }

            let (sector_id, duration) = cycle.instructions[sm.current_instruction];
            let state = sm.state.clone();
            drop(sm);
            match state {
                WateringState::Idle => self.handle_idle(sector_id).await,
                WateringState::Activating(_) => {
                    if let Some(sec) = self.sectors.read().await.get(&sector_id) {
                        self.handle_activating(duration, db, event_type, sec).await
                    }
                }
                WateringState::Deactivating(_) => self.handle_deactivating(sector_id).await,
                _ => {}
            }
        }
    }

    pub async fn is_idle(&self) -> bool {
        self.state_machine.read().await.is_idle()
    }
}

pub fn load_sectors_into_hashmap(sectors: Vec<SectorInfo>) -> HashMap<u32, SectorInfo> {
    let sectors = sectors
        .iter()
        .map(|sector| {
            let mut sec = sector.clone();
            sec.progress = 0.;
            (sector.id, sec)
        })
        .collect();
    sectors
}

pub async fn run_watering_system<C: SensorController + 'static, D: DatabaseTrait + 'static>(
    app_state: Arc<AppState<C, D>>, rx_signal: Arc<Mutex<broadcast::Receiver<ControlSignal>>>,
) {
    let ws = app_state.watering_system.clone();
    let db_clone = app_state.db.clone();

    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let mut daily_adjustment_done: Option<i64> = None;

    let now = chrono::Utc::now().timestamp();

    // Set up a daily timer for adjustments - Wait until midnight
    let midnight = sod(now) + 86400; // Start of the next day
    let until_midnight = midnight - now;
    let mut daily_timer = tokio::time::interval(Duration::from_secs(until_midnight as u64));
    daily_timer.tick().await;

    loop {
        let now = chrono::Utc::now().timestamp();
        tokio::select! {
            _ = daily_timer.tick() => {
                do_daily_adjustments(&ws, &mut daily_adjustment_done, &db_clone, now).await;
            }
            _ = interval.tick() => {
                handle_signals(&ws, rx_signal.clone()).await;
                execute_active_mode(&ws, db_clone.clone(), now).await;
            }
        }
    }
}

async fn do_daily_adjustments<C: SensorController + 'static, D: DatabaseTrait + 'static>(
    ws: &Arc<WateringSystem<C>>, daily_adjustment_done: &mut Option<i64>, db: &Arc<D>, now: i64,
) {
    // Check if adjustments have already been made for the current day
    let day_start = sod(now); // Calculate start of the current day
    if daily_adjustment_done.map_or(true, |last_day| last_day != day_start) {
        *daily_adjustment_done = Some(now);

        let mut wizard_mode = ws.wizard_mode.write().await;
        wizard_mode.handle_daily_adjustments(ws, db, now).await;

        info!("Daily adjustments completed for {:?}", display_from_ts(day_start));
    }
}

async fn handle_signals<C: SensorController + 'static>(
    watering_system: &Arc<WateringSystem<C>>, rx_signal: Arc<Mutex<broadcast::Receiver<ControlSignal>>>,
) {
    if let Ok(signal) = rx_signal.lock().await.try_recv() {
        let mut state_machine = watering_system.state_machine.write().await;
        watering_system.handle_signal(signal, &mut state_machine).await;
    }
}

async fn execute_active_mode<C, D>(watering_system: &Arc<WateringSystem<C>>, db: Arc<D>, now: i64)
where
    C: SensorController + 'static,
    D: DatabaseTrait + 'static,
{
    let mut active_mode = watering_system.active_mode.write().await;
    match &mut *active_mode {
        ModeEnum::Wizard(wizard_mode) => {
            info!("Wizard Mode: Executing dynamic schedule.");
            wizard_mode.execute(watering_system, now, &db).await;
        }
        ModeEnum::Auto(auto_mode) => {
            let base_time = sod(now); // Start of the current day
            let timeframe = *watering_system.timeframe.read().await;
            // let timeframe = watering_system.timeframe.read().await;
            if timeframe.is_within(now, base_time) {
                info!("Auto Mode: Executing auto schedule.");
                auto_mode.execute(watering_system, &db, now).await;
            }
        }
        ModeEnum::Manual(_) => {
            info!("Manual Mode: No automatic scheduling.");
        }
    }
}

#[cfg(test)]
mod test {}
