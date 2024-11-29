use super::{
    ds::{AppState, ControlSignal, Cycle, EventType, SectorInfo, WateringState},
    mode::ModeEnum,
    mode_auto::ModeAuto,
    mode_manual::ModeManual,
    mode_wizard::ModeWizard,
    schedule::{AllowedTimeframe, Schedule, ScheduleEntry},
    state_machine::WateringStateMachine,
};
use crate::{db::DatabaseTrait, error::AppError, sensors::interface::SensorController, watering::ds::WateringEvent};
use chrono::{DateTime, Local, NaiveDate, NaiveTime};
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
        // TODO: Simulated auto schedule (should be read from the database in a real implementation)
        let auto_schedule = Schedule::new(vec![
            ScheduleEntry {
                day_of_week: chrono::Weekday::Mon,
                start_times: vec![(6, chrono::Duration::minutes(30))], // Example: start time at 6:00, 30 min
            },
            ScheduleEntry { day_of_week: chrono::Weekday::Wed, start_times: vec![(6, chrono::Duration::minutes(30))] },
            ScheduleEntry { day_of_week: chrono::Weekday::Fri, start_times: vec![(6, chrono::Duration::minutes(30))] },
        ]);

        // Initialize modes
        let auto_mode = ModeAuto::new(Cycle::default(), auto_schedule);
        let manual_mode = ModeManual::new(Cycle::default());

        // Initialize ModeWizard with an empty schedule
        let wizard_schedule = Schedule::new(vec![]); // Starts with an empty schedule, to be recalculated dynamically
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
        &self, duration: chrono::Duration, db: &Arc<D>, event_type: EventType, sector: &SectorInfo,
    ) {
        let mut sm = self.state_machine.write().await;
        sm.state = WateringState::Watering(sector.id, duration);
        info!("State transitioned to Watering for sector {} with duration {:?}", sector.id, duration);

        let controller_clone = self.controller.clone();
        let db_clone = db.clone();
        let sector_clone = sector.clone();
        let sector_id_clone = sector.id;
        let total_duration_secs = duration.num_seconds();
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

    pub fn calculate_deep_watering_schedule(
        &self, sectors: &[SectorInfo], timeframe: AllowedTimeframe, weekly_target: f64, daily_et: f64, days_remaining: u32,
    ) -> Vec<(NaiveTime, Vec<(u32, chrono::Duration)>)> {
        let total_sectors = sectors.len();
        if total_sectors == 0 || days_remaining == 0 {
            return vec![]; // No sectors or no days remaining
        }

        let start = timeframe.start;
        let end = timeframe.end;
        let total_available_duration = (end - start).num_seconds().max(1);

        // Adjust each sector's remaining water needs based on ET and percolation
        let water_needed_per_day = (weekly_target - daily_et * (7 - days_remaining) as f64).max(0.0) / days_remaining as f64;

        let mut sector_durations: Vec<(u32, chrono::Duration)> = sectors
            .iter()
            .filter_map(|sector| {
                let adjusted_target = (water_needed_per_day - sector.percolation_rate).max(0.0);
                let remaining_water = adjusted_target - sector.progress;

                if remaining_water > 0.0 {
                    let duration_seconds = ((remaining_water / sector.sprinkler_debit) * 3600.0).ceil() as i64;
                    let duration = chrono::Duration::seconds(duration_seconds.min(sector.max_duration.num_seconds() as i64));
                    Some((sector.id, duration))
                } else {
                    None
                }
            })
            .collect();

        // Sort sectors by their remaining water need (descending) for prioritization
        sector_durations.sort_by(|(_, d1), (_, d2)| d2.num_seconds().cmp(&d1.num_seconds()));

        let mut schedule = vec![];
        let mut remaining_duration = total_available_duration;

        // Allocate watering times across sectors
        let mut current_time = start;
        while remaining_duration > 0 && !sector_durations.is_empty() {
            let mut cycle_durations = vec![];
            let mut cycle_time_used = 0;

            while !sector_durations.is_empty() && cycle_time_used < remaining_duration {
                let (sector_id, duration) = sector_durations.remove(0);

                if cycle_time_used + duration.num_seconds() <= remaining_duration {
                    cycle_durations.push((sector_id, duration));
                    cycle_time_used += duration.num_seconds();
                } else {
                    // Partially water the sector
                    let partial_duration = chrono::Duration::seconds(remaining_duration - cycle_time_used);
                    cycle_durations.push((sector_id, partial_duration));
                    sector_durations.insert(0, (sector_id, duration - partial_duration));
                    break;
                }
            }

            schedule.push((current_time, cycle_durations));
            current_time = current_time + chrono::Duration::seconds(cycle_time_used);
            remaining_duration -= cycle_time_used;
        }

        schedule
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
    app_state: Arc<AppState<C, D>>, rx_signal: Arc<Mutex<broadcast::Receiver<ControlSignal>>>,
) {
    let ws = app_state.watering_system.clone();
    let db_clone = app_state.db.clone();

    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let mut daily_adjustment_done: Option<NaiveDate> = None;
    loop {
        interval.tick().await;
        let now = Local::now();

        handle_signals(&ws, rx_signal.clone()).await;

        do_daily_adjustments(&ws, &mut daily_adjustment_done, &db_clone, now).await;

        execute_active_mode(&ws, db_clone.clone(), now).await;
    }
}

async fn do_daily_adjustments<C: SensorController + 'static, D: DatabaseTrait + 'static>(
    ws: &Arc<WateringSystem<C>>, daily_adjustment_done: &mut Option<NaiveDate>, db: &Arc<D>, now: DateTime<Local>,
) {
    // Perform daily ET adjustment
    let current_date = now.date_naive();

    // Check if adjustments have already been made for the current day
    if daily_adjustment_done.map_or(true, |last_date| last_date != current_date) {
        *daily_adjustment_done = Some(current_date);

        let mut wizard_mode = ws.wizard_mode.write().await;
        wizard_mode.handle_daily_adjustments(ws, db).await;
        info!("Daily adjustments completed for {:?}", current_date);
    } 
    // else {
    //     debug!("Daily adjustments already completed for {:?}", current_date);
    // }
}

async fn handle_signals<C: SensorController + 'static>(
    watering_system: &Arc<WateringSystem<C>>, rx_signal: Arc<Mutex<broadcast::Receiver<ControlSignal>>>,
) {
    if let Ok(signal) = rx_signal.lock().await.try_recv() {
        let mut state_machine = watering_system.state_machine.write().await;
        watering_system.handle_signal(signal, &mut state_machine).await;
    }
}

async fn execute_active_mode<C, D>(watering_system: &Arc<WateringSystem<C>>, db: Arc<D>, now: DateTime<Local>)
where
    C: SensorController + 'static,
    D: DatabaseTrait + 'static,
{
    let mut active_mode = watering_system.active_mode.write().await;
    let time = now.time();
    match &mut *active_mode {
        ModeEnum::Wizard(wizard_mode) => {
            if let Some(next_start) = wizard_mode.calculate_next_start(time, watering_system.timeframe.read().await.clone()) {
                if time >= next_start {
                    info!("Wizard Mode: Executing dynamic schedule.");
                    wizard_mode.execute(watering_system, now.date_naive(), &db).await;
                }
            }
        }
        ModeEnum::Auto(auto_mode) => {
            if time >= watering_system.timeframe.read().await.start && time < watering_system.timeframe.read().await.end {
                info!("Auto Mode: Executing auto schedule.");
                auto_mode.execute(watering_system, &db, now.to_utc()).await;
            }
        }
        ModeEnum::Manual(_) => {
            info!("Manual Mode: No automatic scheduling.");
        }
    }
}

#[cfg(test)]
mod test {

    use crate::watering::{ds::SectorInfo, mode_wizard::ModeWizard, schedule::Schedule};

    #[tokio::test]
    async fn test_et_adjustments() {
        let schedule = Schedule::new(vec![]); // Create an empty schedule for the wizard mode
        let mut wizard_mode = ModeWizard::new(schedule);
        let mut sectors = vec![SectorInfo {
            id: 1,
            weekly_target: 3.0,
            progress: 0.5,
            sprinkler_debit: 1.0,
            percolation_rate: 0.5,
            max_duration: chrono::Duration::minutes(30),
        }];

        wizard_mode.adjust_progress_for_et(&mut sectors.iter_mut().collect::<Vec<&mut SectorInfo>>(), 0.)
    }
}
