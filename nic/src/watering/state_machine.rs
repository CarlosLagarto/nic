use std::{sync::Arc, time::Duration};

use super::{
    ds::{AppState, ControlSignal, Cycle, WateringState},
    interface::SensorController,
    mode::ModeEnum,
    mode_auto::ModeAuto,
    mode_manual::ModeManual,
    mode_wizard::ModeWizard,
    schedule::AllowedTimeframe,
};
use crate::{db::Database, watering::ds::WateringEvent};
use chrono::{Local, NaiveTime};
use tokio::sync::{broadcast, Mutex, RwLock};

pub struct WateringSystem<C: SensorController> {
    pub state_machine: Arc<RwLock<WateringStateMachine>>,
    pub auto_mode: Arc<RwLock<ModeAuto>>,
    pub manual_mode: Arc<RwLock<ModeManual>>,
    pub wizard_mode: Arc<RwLock<ModeWizard>>,
    pub active_mode: Arc<RwLock<ModeEnum>>, // Tracks the currently active mode
    pub controller: Arc<C>,                 // Sensor controller (mockable)
}

impl<C: SensorController> WateringSystem<C> {
    pub async fn new(controller: Arc<C>) -> Arc<Self> {
        let timeframe = AllowedTimeframe {
            start: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
        };

        let state_machine = WateringStateMachine::new(timeframe.clone());

        // TODO: Load modes (e.g., AutoMode, ManualMode, WizardMode) from the database
        let auto_mode = ModeAuto::new(Cycle::default(), timeframe.clone());
        let manual_mode = ModeManual::new(Cycle::default());
        let wizard_mode = ModeWizard::new(vec![], timeframe);

        Arc::new(WateringSystem {
            state_machine: Arc::new(RwLock::new(state_machine)),
            controller,
            auto_mode: Arc::new(RwLock::new(auto_mode.clone())),
            manual_mode: Arc::new(RwLock::new(manual_mode)),
            wizard_mode: Arc::new(RwLock::new(wizard_mode)),
            active_mode: Arc::new(RwLock::new(ModeEnum::Auto(auto_mode))),
        })
    }

    pub async fn switch_mode(&self, new_mode: ModeEnum) {
        let mut active_mode = self.active_mode.write().await;

        match &new_mode {
            ModeEnum::Auto(_) => {
                println!("Switching to Auto Mode");
            }
            ModeEnum::Manual(_) => {
                println!("Switching to Manual Mode");
            }
            ModeEnum::Wizard(_) => {
                println!("Switching to Wizard Mode");
            }
        }

        *active_mode = new_mode;
    }
}

pub async fn run_watering_system<C: SensorController + 'static>(
    app_state: Arc<AppState<C>>,
    rx_signal: Arc<Mutex<broadcast::Receiver<ControlSignal>>>,
) {
    let watering_system_clone = app_state.watering_system.clone();
    let db_clone = app_state.db.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await; // Example interval
            let mut wizard_mode = watering_system_clone.wizard_mode.write().await;
            let mut state_machine = watering_system_clone.state_machine.write().await;
            wizard_mode.update(&mut state_machine, &db_clone).await;
        }
    });

    let mut interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        interval.tick().await;

        // Handle control signals
        if let Ok(signal) = rx_signal.lock().await.try_recv() {
            match signal {
                ControlSignal::SwitchToAuto => {
                    let auto_mode = app_state.watering_system.auto_mode.read().await.clone();
                    app_state
                        .watering_system
                        .switch_mode(ModeEnum::Auto(auto_mode))
                        .await;
                }
                ControlSignal::SwitchToManual => {
                    let manual_mode = app_state.watering_system.manual_mode.read().await.clone();
                    app_state
                        .watering_system
                        .switch_mode(ModeEnum::Manual(manual_mode))
                        .await;
                }
                ControlSignal::SwitchToWizard => {
                    let wizard_mode = app_state.watering_system.wizard_mode.read().await.clone();
                    app_state
                        .watering_system
                        .switch_mode(ModeEnum::Wizard(wizard_mode))
                        .await;
                }
                ControlSignal::Environmental(env_signal) => {
                    let mut active_mode = app_state.watering_system.active_mode.write().await;
                    if let ModeEnum::Wizard(wizard_mode) = &mut *active_mode {
                        let mut state_machine =
                            app_state.watering_system.state_machine.write().await;
                        wizard_mode.handle_signal(env_signal, &mut state_machine);
                    }
                }
                ControlSignal::StopMachine => {
                    let mut state_machine = app_state.watering_system.state_machine.write().await;
                    state_machine.state = WateringState::Idle;
                    state_machine.cycle = None;
                }
                ControlSignal::Weather(_weather) => {} //TODO:
                ControlSignal::DevicesState(_state) => {} //TODO:
            }
        }

        // Execute the current mode
        {
            let current_time = Local::now().time();

            let mut active_mode = app_state.watering_system.active_mode.write().await;
            let mut state_machine = app_state.watering_system.state_machine.write().await;
            active_mode
                .execute(
                    &mut state_machine,
                    app_state.db.clone(),
                    current_time,
                    &app_state.watering_system.controller,
                )
                .await;
        }
    }
}

pub struct WateringStateMachine {
    pub state: WateringState,
    pub cycle: Option<Cycle>,
    pub current_instruction: usize,
}

impl WateringStateMachine {
    pub fn new(_allowed_timeframe: AllowedTimeframe) -> Self {
        Self {
            state: WateringState::Idle,
            cycle: None,
            current_instruction: 0,
        }
    }

    pub fn start_cycle(&mut self, cycle: Cycle) {
        self.cycle = Some(cycle);
        self.current_instruction = 0;
        self.state = WateringState::Idle;
    }

    pub async fn update<C: SensorController>(
        &mut self,
        db: Database,
        event_type: &str,
        controller: &Arc<C>,
    ) {
        if let Some(cycle) = &self.cycle {
            if self.current_instruction >= cycle.instructions.len() {
                println!("Cycle completed.");
                self.cycle = None;
                self.state = WateringState::Idle;
                return;
            }

            let (sector_id, duration) = cycle.instructions[self.current_instruction];
            match &self.state {
                WateringState::Idle => {
                    println!("Activating sector {}", sector_id);
                    self.state = WateringState::Activating(sector_id);
                    controller.activate_sector(sector_id).await;
                }
                WateringState::Activating(_) => {
                    println!("Watering sector {} for {:?}", sector_id, duration);
                    self.state = WateringState::Watering(sector_id);

                    // simulate watering
                    std::thread::sleep(std::time::Duration::from_secs(
                        duration.num_seconds() as u64
                    ));

                    let water_applied = (duration.num_minutes() as f64 / 60.0) * 2.5;
                    _ = db.log_watering_event(WateringEvent::new(
                        Some(cycle.id),
                        sector_id,
                        chrono::Local::now().to_rfc3339(),
                        duration,
                        water_applied,
                        event_type.to_owned(),
                    ));
                    self.state = WateringState::Deactivating(sector_id);
                }
                WateringState::Deactivating(_) => {
                    println!("Deactivating sector {}", sector_id);
                    controller.deactivate_sector(sector_id).await;
                    self.current_instruction += 1;
                    self.state = WateringState::Idle;
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod state_machine_tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_start_cycle() {
        let timeframe = AllowedTimeframe {
            start: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
        };
        let mut state_machine = WateringStateMachine::new(timeframe);

        let cycle = Cycle {
            id: 1,
            instructions: vec![(1, Duration::minutes(30))],
        };
        state_machine.start_cycle(cycle.clone());

        assert_eq!(state_machine.cycle, Some(cycle));
        assert_eq!(state_machine.current_instruction, 0);
        assert_eq!(state_machine.state, WateringState::Idle);
    }
}
