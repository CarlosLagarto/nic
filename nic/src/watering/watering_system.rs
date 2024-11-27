use std::{collections::HashMap, sync::Arc, time::Duration};

use super::{
    ds::{AppState, ControlSignal, Cycle, EventType, SectorInfo, WateringState},
    mode::ModeEnum,
    mode_auto::ModeAuto,
    mode_manual::ModeManual,
    mode_wizard::ModeWizard,
    schedule::AllowedTimeframe,
    state_machine::WateringStateMachine,
};
use crate::{
    db::DatabaseTrait, error::AppError, sensors::interface::SensorController,
    watering::ds::WateringEvent,
};
use chrono::{Local, NaiveTime};
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
    pub async fn new<D: DatabaseTrait>(
        controller: Arc<C>,
        db: Arc<D>,
    ) -> Result<Arc<Self>, AppError> {
        let timeframe = Arc::new(RwLock::new(AllowedTimeframe {
            start: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
        }));
        // Initialize all progress to 0
        let sectors: Vec<SectorInfo> = db.load_sectors()?;
        let sectors = Arc::new(RwLock::new(load_sectors(sectors)));

        let state_machine = WateringStateMachine::new();

        // TODO: Load modes (e.g., AutoMode, ManualMode, WizardMode) from the database
        let (auto_mode, manual_mode, wizard_mode) = Self::initialize_modes();

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

    fn initialize_modes() -> (ModeAuto, ModeManual, ModeWizard) {
        let auto_mode = ModeAuto::new(Cycle::default());
        let manual_mode = ModeManual::new(Cycle::default());
        let wizard_mode = ModeWizard::new();
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
            ControlSignal::Weather(_weather) => {} //TODO:
            ControlSignal::DevicesState(_state) => {} //TODO:
                                                    // _ => info!("Unhandled signal {:?}", signal),
        }
    }

    async fn handle_idle(&self, sector_id: u32) {
        let mut sm = self.state_machine.write().await;
        sm.state = WateringState::Activating(sector_id);
        self.controller.activate_sector(sector_id).await;
        info!("State transitioned to Activating for sector {}", sector_id);
    }

    pub async fn handle_activating<D: DatabaseTrait + 'static>(
        &self,
        duration: chrono::Duration,
        db: &Arc<D>,
        event_type: EventType,
        sector: &SectorInfo,
    ) {
        let mut sm = self.state_machine.write().await;
        sm.state = WateringState::Watering(sector.id, duration);
        info!(
            "State transitioned to Watering for sector {} with duration {:?}",
            sector.id, duration
        );

        let controller_clone = self.controller.clone();
        let db_clone = db.clone();
        let sector_clone = sector.clone();
        let sector_id_clone = sector.id;
        let total_duration_secs = duration.num_seconds();
        // let progress = self.progress.clone();
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
                        info!(
                            "Sector {} watering progress: {:.2} cm",
                            sector_id_clone, sec.progress
                        );
                    }
                }
                // Handle potential interruptions
                if elapsed_secs >= total_duration_secs {
                    info!("Completed watering for sector {}", sector_id_clone);
                    let water_applied =
                        (elapsed_secs as f64 / 3600.0) * sector_clone.sprinkler_debit; // Final water applied
                                                                                       // let water_applied = (duration_clone.num_minutes() as f64 / 60.0) * 2.5;
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
        info!(
            "State transitioned to Idle after deactivating sector {}",
            sector_id
        );
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

pub fn load_sectors(sectors: Vec<SectorInfo>) -> HashMap<u32, SectorInfo> {
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
    app_state: Arc<AppState<C, D>>,
    rx_signal: Arc<Mutex<broadcast::Receiver<ControlSignal>>>,
) {
    let watering_system_clone2 = app_state.watering_system.clone();
    let db_clone = app_state.db.clone();

    let mut interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        interval.tick().await;
        let now = Local::now().time();
        // Handle control signals
        if let Ok(signal) = rx_signal.lock().await.try_recv() {
            let mut state_machine = watering_system_clone2.state_machine.write().await;
            app_state
                .watering_system
                .handle_signal(signal, &mut state_machine)
                .await;
        }
        // Execute the current mode and handle mode-specific scheduling
        {
            let ws = app_state.watering_system.clone();
            let mut active_mode = ws.active_mode.write().await;
            let timeframe = ws.timeframe.read().await.clone();

            match &mut *active_mode {
                ModeEnum::Wizard(wizard_mode) => {
                    if let Some(next_start) = wizard_mode.calculate_next_start(now, timeframe) {
                        if now >= next_start {
                            info!("Wizard Mode: Executing dynamic schedule.");
                            // let sm = ws.state_machine.write().await;
                            wizard_mode.execute(&ws, now, &db_clone).await;
                        }
                    }
                }
                ModeEnum::Auto(auto_mode) => {
                    let auto_start_time = timeframe.start;
                    if now >= auto_start_time && now < timeframe.end {
                        info!("Auto Mode: Executing auto schedule.");
                        auto_mode.execute(&(*ws), &app_state.db, now).await;
                    }
                }
                ModeEnum::Manual(_) => {
                    info!("Manual Mode: No automatic scheduling.");
                }
            }
        }
    }
}
